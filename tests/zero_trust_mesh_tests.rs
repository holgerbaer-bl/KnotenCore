// =============================================================================
// Sprint 327: Zero-Trust Mesh Phase 1 Integration Tests
// =============================================================================

use aether_compiler::crypto_ed25519::Ed25519KeyPair;
use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::RpcServer;
use serde_json::Value;

fn parse_response(json_str: &str) -> Value {
    serde_json::from_str(json_str).expect("Valid JSON-RPC response")
}

#[test]
fn test_zero_trust_ed25519_envelope_signing_and_peer_verification() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-alpha",
        "127.0.0.1:9090",
        Some("secret-token".to_string()),
    );
    server.enable_zero_trust();

    let client_keypair = Ed25519KeyPair::generate();
    let client_pubkey = client_keypair.public_key_hex();
    let nonce = "nonce-1001";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let message = format!("{}:{}:node-beta", now, nonce);
    let sig = client_keypair.sign_hex(message.as_bytes());

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_verify_peer",
        "params": {
            "peer_node_id": "node-beta",
            "sender_node_id": "node-beta",
            "public_key": client_pubkey,
            "signature": sig,
            "nonce": nonce,
            "timestamp": now
        },
        "id": 1
    });

    let resp = parse_response(&server.dispatch_request(&req.to_string()));
    assert_eq!(resp["result"]["status"], "ok");
    assert_eq!(resp["result"]["verified"], true);
    assert_eq!(resp["result"]["peer_node_id"], "node-beta");
    assert_eq!(resp["result"]["peer_public_key"], client_pubkey);
    assert_eq!(resp["result"]["local_public_key"], server.public_key_hex());
}

#[test]
fn test_zero_trust_rejects_invalid_ed25519_signature() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-alpha",
        "127.0.0.1:9090",
        None,
    );
    server.enable_zero_trust();

    let client_keypair = Ed25519KeyPair::generate();
    let client_pubkey = client_keypair.public_key_hex();
    let nonce = "nonce-1002";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Sign a DIFFERENT message (tampered)
    let bad_message = format!("{}:{}:tampered-node", now, nonce);
    let bad_sig = client_keypair.sign_hex(bad_message.as_bytes());

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_ping",
        "params": {
            "sender_node_id": "node-beta",
            "public_key": client_pubkey,
            "signature": bad_sig,
            "nonce": nonce,
            "timestamp": now
        },
        "id": 1
    });

    let resp = parse_response(&server.dispatch_request(&req.to_string()));
    assert!(resp["error"].is_object());
    assert_eq!(resp["error"]["code"], -32001);
    assert!(
        resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Invalid Ed25519 signature")
    );
}

#[test]
fn test_zero_trust_blocks_downgrade_attempts() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-alpha",
        "127.0.0.1:9090",
        Some("secret-token".to_string()),
    );
    server.enable_zero_trust();

    // Attempt plain legacy HMAC token submission without Ed25519 envelope
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_ping",
        "params": {
            "mesh_auth_token": "secret-token",
            "sender_node_id": "attacker-node"
        },
        "id": 1
    });

    let resp = parse_response(&server.dispatch_request(&req.to_string()));
    assert!(resp["error"].is_object());
    assert_eq!(resp["error"]["code"], -32001);
    assert!(
        resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("rejected in zero-trust mode")
    );
}

#[test]
fn test_zero_trust_replay_attack_prevention() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-alpha",
        "127.0.0.1:9090",
        None,
    );
    server.enable_zero_trust();

    let client_keypair = Ed25519KeyPair::generate();
    let client_pubkey = client_keypair.public_key_hex();
    let nonce = "nonce-replay-99";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let message = format!("{}:{}:node-gamma", now, nonce);
    let sig = client_keypair.sign_hex(message.as_bytes());

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_ping",
        "params": {
            "sender_node_id": "node-gamma",
            "public_key": client_pubkey,
            "signature": sig,
            "nonce": nonce,
            "timestamp": now
        },
        "id": 1
    });

    // 1st attempt: success
    let resp1 = parse_response(&server.dispatch_request(&req.to_string()));
    assert_eq!(resp1["result"]["status"], "ok");

    // 2nd attempt with same nonce: rejected as replay
    let resp2 = parse_response(&server.dispatch_request(&req.to_string()));
    assert!(resp2["error"].is_object());
    assert!(
        resp2["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Replayed nonce detected")
    );

    // Expired timestamp (> 30s)
    let expired_ts = now.saturating_sub(45);
    let expired_nonce = "nonce-expired-100";
    let exp_msg = format!("{}:{}:node-gamma", expired_ts, expired_nonce);
    let exp_sig = client_keypair.sign_hex(exp_msg.as_bytes());

    let exp_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_ping",
        "params": {
            "sender_node_id": "node-gamma",
            "public_key": client_pubkey,
            "signature": exp_sig,
            "nonce": expired_nonce,
            "timestamp": expired_ts
        },
        "id": 2
    });

    let resp_exp = parse_response(&server.dispatch_request(&exp_req.to_string()));
    assert!(resp_exp["error"].is_object());
    assert!(
        resp_exp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("timestamp expired or invalid")
    );
}
