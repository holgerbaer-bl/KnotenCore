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
    assert_eq!(KNC_PROTOCOL_VERSION, "v2.24.16");
}

#[test]
fn test_unilateral_reelection_rejected() {
    let server = RpcServer::new(aether_compiler::executor::AgentPermissions::default());
    server.set_revoked_keys_path(None);

    // Register a peer so quorum requires multi-node consensus (total_active_nodes = 2)
    let reg_peer = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 0,
        "method": "knc_mesh_peers",
        "params": {
            "action": "register",
            "peer": {
                "node_id": "peer_other",
                "address": "127.0.0.1:9999",
                "status": "Active",
                "capabilities": ["worker"]
            }
        }
    });
    server.dispatch_request(&reg_peer.to_string());

    // Initial election
    let req1 = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_swarm_elect",
        "params": { "candidate_node_id": "node_alpha" }
    });
    let _resp1 = server.dispatch_request(&req1.to_string());

    // Subsequent re-election attempt without swarm consensus must be rejected (-32001)
    let req2 = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "knc_swarm_elect",
        "params": { "candidate_node_id": "node_beta", "force": true }
    });
    let resp2 = server.dispatch_request(&req2.to_string());
    assert!(
        resp2.contains("-32001"),
        "Unilateral re-election was not rejected: {}",
        resp2
    );
    assert!(
        resp2.contains("Leader is already elected")
            || resp2.contains("Unauthorized")
            || resp2.contains("Quorum")
    );
}

#[test]
fn test_client_cannot_bypass_via_test_harness_param() {
    let server = RpcServer::new(aether_compiler::executor::AgentPermissions::default());
    server.set_revoked_keys_path(None);

    // Register a peer so quorum requires multi-node consensus (total_active_nodes = 2)
    let reg_peer = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 0,
        "method": "knc_mesh_peers",
        "params": {
            "action": "register",
            "peer": {
                "node_id": "peer_other",
                "address": "127.0.0.1:9999",
                "status": "Active",
                "capabilities": ["worker"]
            }
        }
    });
    server.dispatch_request(&reg_peer.to_string());

    // Set initial leader
    let req1 = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_swarm_elect",
        "params": { "candidate_node_id": "node_alpha" }
    });
    let _ = server.dispatch_request(&req1.to_string());

    // Attempting to bypass using allow_test_harness parameter MUST fail with -32001
    let req_bypass = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "knc_swarm_elect",
        "params": {
            "candidate_node_id": "node_rogue",
            "force": true,
            "allow_test_harness": true,
            "test_harness": true
        }
    });
    let resp = server.dispatch_request(&req_bypass.to_string());
    assert!(
        resp.contains("-32001"),
        "Client bypass via allow_test_harness succeeded unexpectedly: {}",
        resp
    );
}

#[test]
fn test_oversized_session_id_rejected_on_all_endpoints() {
    let server = RpcServer::new(aether_compiler::executor::AgentPermissions::default());
    let oversized_session = "s".repeat(257);

    // knc_compile
    let ast = knoten_core_types::ast::Node::IntLiteral(42);
    let req_compile = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_compile",
        "params": { "ast": ast, "session_id": oversized_session }
    });
    let resp_compile = server.dispatch_request(&req_compile.to_string());
    assert!(
        resp_compile.contains("-32602"),
        "Oversized session_id on knc_compile not rejected: {}",
        resp_compile
    );
    assert!(resp_compile.contains("exceeds maximum length"));

    // knc_execute
    let req_exec = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "knc_execute",
        "params": { "ast": ast, "session_id": oversized_session }
    });
    let resp_exec = server.dispatch_request(&req_exec.to_string());
    assert!(
        resp_exec.contains("-32602"),
        "Oversized session_id on knc_execute not rejected: {}",
        resp_exec
    );

    // knc_inspect_state
    let req_inspect = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "knc_inspect_state",
        "params": { "session_id": oversized_session }
    });
    let resp_inspect = server.dispatch_request(&req_inspect.to_string());
    assert!(
        resp_inspect.contains("-32602"),
        "Oversized session_id on knc_inspect_state not rejected: {}",
        resp_inspect
    );
}
