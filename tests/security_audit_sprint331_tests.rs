use aether_compiler::rpc::{
    KNC_PROTOCOL_VERSION, MAX_BODY_BYTES, MAX_WS_PAYLOAD, RpcServer, hmac_sha256, read_ws_frame,
};
use aether_compiler::vm::machine::VM;
use knoten_core_types::opcode::OpCode;
use std::io::Cursor;

#[test]
fn test_max_recursion_depth_guard() {
    let mut vm = VM::new();
    let perms = aether_compiler::executor::AgentPermissions::default();

    // Create a recursive call chain exceeding 512 frames (513 calls)
    let mut instructions = Vec::new();
    for _ in 0..513 {
        instructions.push(OpCode::Call(0, 0));
    }
    instructions.push(OpCode::Return);

    let constants = vec![];
    let result = vm.run(&instructions, &constants, &perms, None);

    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("ERR_CALL_DEPTH_EXCEEDED") || err_msg.contains("Call depth exceeded"),
        "Expected call depth limit error, got: {}",
        err_msg
    );
}

#[test]
fn test_rpc_oversized_tcp_body_rejected() {
    let server = RpcServer::new(aether_compiler::executor::AgentPermissions::default());
    let oversized_payload = "a".repeat(MAX_BODY_BYTES + 100);
    let resp_json = server.dispatch_request(&oversized_payload);

    assert!(resp_json.contains("-32700"));
    assert!(resp_json.contains("Parse Error: Request payload size"));
}

#[test]
fn test_ws_oversized_frame_rejected() {
    // Construct a WebSocket frame declaring a payload length of MAX_WS_PAYLOAD + 100
    let payload_len: u64 = (MAX_WS_PAYLOAD + 100) as u64;
    let mut frame_bytes = vec![0x81, 127]; // Text frame, unmasked, 64-bit extended payload length
    frame_bytes.extend_from_slice(&payload_len.to_be_bytes());

    let mut cursor = Cursor::new(frame_bytes);
    let result = read_ws_frame(&mut cursor);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(err.to_string().contains("WebSocket payload size"));
}

#[test]
fn test_hmac_replay_within_window() {
    let auth_token = "secret_mesh_token_12345";
    let server = RpcServer::with_mesh(
        aether_compiler::executor::AgentPermissions::default(),
        "node_alpha",
        "127.0.0.1:9090",
        Some(auth_token.to_string()),
    );

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let nonce = "unique_nonce_req_99";
    let sender = "node_alpha";
    let message = format!("{}:{}", timestamp, sender);
    let sig = hmac_sha256(auth_token.as_bytes(), message.as_bytes());

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_mesh_ping",
        "params": {
            "mesh_auth_signature": sig,
            "nonce": nonce,
            "timestamp": timestamp,
            "sender_node_id": sender
        }
    });

    // First request must succeed
    let first_resp = server.dispatch_request(&req.to_string());
    assert!(
        first_resp.contains("\"result\""),
        "First request failed: {}",
        first_resp
    );

    // Replaying the exact same nonce within the 30s replay window must be rejected
    let second_resp = server.dispatch_request(&req.to_string());
    assert!(
        second_resp.contains("-32001"),
        "Replayed request was not rejected: {}",
        second_resp
    );
    assert!(second_resp.contains("Replayed nonce detected"));
}

#[test]
fn test_version_assertion_sprint331() {
    assert_eq!(KNC_PROTOCOL_VERSION, "v2.21.1-security");
}
