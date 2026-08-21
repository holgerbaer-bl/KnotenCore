use aether_compiler::crypto_ed25519::Ed25519KeyPair;
use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::RpcServer;
use aether_compiler::rpc::types::KNC_PROTOCOL_VERSION;
use knoten_core_types::ast::Node;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn test_version_assertion_sprint356() {
    assert_eq!(
        KNC_PROTOCOL_VERSION, "v2.24.19",
        "Protocol version must be synchronized to v2.24.19 for Sprint 356"
    );
}

#[test]
fn test_eval_dual_auth_gating_unauthenticated_rejected() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-test-1",
        "127.0.0.1:0",
        Some("mesh-secret-123".to_string()),
    );
    server.enable_zero_trust();

    // 1. Missing auth credentials -> -32001
    let ast = Node::Add(
        Box::new(Node::IntLiteral(10)),
        Box::new(Node::IntLiteral(32)),
    );

    let req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_eval_dual",
        "params": {
            "session_id": "test-session",
            "ast": ast
        }
    });

    let resp_str = server.dispatch_request(&req.to_string());
    let resp: serde_json::Value = serde_json::from_str(&resp_str).expect("Valid JSON response");
    assert!(
        resp.get("error").is_some(),
        "Unauthenticated request must return an error"
    );
    let err_code = resp["error"]["code"].as_i64().unwrap();
    assert_eq!(err_code, -32001, "Expected -32001 for unauthorized access");

    // 2. Invalid token -> -32001
    let req_invalid = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "knc_eval_dual",
        "params": {
            "session_id": "test-session",
            "ast": ast,
            "mesh_auth_token": "wrong-token"
        }
    });

    let resp_str2 = server.dispatch_request(&req_invalid.to_string());
    let resp2: serde_json::Value = serde_json::from_str(&resp_str2).expect("Valid JSON response");
    assert_eq!(
        resp2["error"]["code"].as_i64().unwrap(),
        -32001,
        "Invalid token must return -32001"
    );
}

#[test]
fn test_eval_dual_ed25519_zero_trust_auth_success() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-test-1",
        "127.0.0.1:0",
        Some("mesh-secret-123".to_string()),
    );
    server.enable_zero_trust();

    let client_keypair = Ed25519KeyPair::generate();
    let client_pubkey_hex = client_keypair.public_key_hex();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let nonce = format!("nonce-dual-eval-{}", now);
    let canonical_msg = format!("{}:{}:{}", now, nonce, "node-client-1");
    let sig_hex = client_keypair.sign_hex(canonical_msg.as_bytes());

    let ast = Node::Add(
        Box::new(Node::IntLiteral(40)),
        Box::new(Node::IntLiteral(2)),
    );

    let req = json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "knc_eval_dual",
        "params": {
            "session_id": "verified-session",
            "ast": ast,
            "public_key": client_pubkey_hex,
            "signature": sig_hex,
            "timestamp": now,
            "nonce": nonce,
            "sender_node_id": "node-client-1"
        }
    });

    let resp_str = server.dispatch_request(&req.to_string());
    let resp: serde_json::Value = serde_json::from_str(&resp_str).expect("Valid JSON response");

    assert!(
        resp.get("result").is_some(),
        "Valid zero-trust request must succeed: {}",
        resp_str
    );
    assert_eq!(resp["result"]["status"], "ok");
    assert_eq!(resp["result"]["execution_mode"], "dual_verified");
    assert_eq!(resp["result"]["result"], 42);
    assert!(resp["result"]["fault"].is_null());
    assert!(
        resp["result"]["telemetry"]["vm_gas_consumed"]
            .as_u64()
            .is_some()
    );
}

#[test]
fn test_eval_dual_legacy_hmac_auth_success() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-test-1",
        "127.0.0.1:0",
        Some("mesh-secret-123".to_string()),
    );

    let ast = Node::Mul(Box::new(Node::IntLiteral(6)), Box::new(Node::IntLiteral(7)));

    let req = json!({
        "jsonrpc": "2.0",
        "id": 20,
        "method": "knc_eval_dual",
        "params": {
            "session_id": "legacy-session",
            "ast": ast,
            "mesh_auth_token": "mesh-secret-123"
        }
    });

    let resp_str = server.dispatch_request(&req.to_string());
    let resp: serde_json::Value = serde_json::from_str(&resp_str).expect("Valid JSON response");

    assert!(
        resp.get("result").is_some(),
        "Valid HMAC token must succeed: {}",
        resp_str
    );
    assert_eq!(resp["result"]["status"], "ok");
    assert_eq!(resp["result"]["execution_mode"], "dual_verified");
    assert_eq!(resp["result"]["result"], 42);
}

#[test]
fn test_eval_dual_symmetrical_fault_parity() {
    let server = RpcServer::new(AgentPermissions::default());

    // Division by zero triggers symmetrical fault across both engines
    let ast = Node::Div(
        Box::new(Node::IntLiteral(100)),
        Box::new(Node::IntLiteral(0)),
    );

    let req = json!({
        "jsonrpc": "2.0",
        "id": 30,
        "method": "knc_eval_dual",
        "params": {
            "session_id": "div-zero-session",
            "ast": ast
        }
    });

    let resp_str = server.dispatch_request(&req.to_string());
    let resp: serde_json::Value = serde_json::from_str(&resp_str).expect("Valid JSON response");

    assert!(
        resp.get("result").is_some(),
        "Symmetrical fault is a valid dual execution result: {}",
        resp_str
    );
    assert_eq!(resp["result"]["status"], "ok");
    assert_eq!(resp["result"]["execution_mode"], "dual_verified");
    assert!(resp["result"]["result"].is_null());
    assert_eq!(resp["result"]["fault"]["category"], "DivisionByZero");
}

#[test]
fn test_eval_dual_isolate_quota_params() {
    let server = RpcServer::new(AgentPermissions::default());

    let ast = Node::Add(
        Box::new(Node::IntLiteral(10)),
        Box::new(Node::IntLiteral(20)),
    );

    let req = json!({
        "jsonrpc": "2.0",
        "id": 40,
        "method": "knc_eval_dual",
        "params": {
            "session_id": "quota-session",
            "ast": ast,
            "isolate_quota": {
                "max_instructions": 500000,
                "max_memory_bytes": 8388608,
                "execution_timeout_ms": 3000
            }
        }
    });

    let resp_str = server.dispatch_request(&req.to_string());
    let resp: serde_json::Value = serde_json::from_str(&resp_str).expect("Valid JSON response");

    assert_eq!(resp["result"]["status"], "ok");
    assert_eq!(resp["result"]["execution_mode"], "dual_verified");
    assert_eq!(resp["result"]["result"], 30);
}

#[test]
fn test_eval_dual_missing_ast_returns_32602() {
    let server = RpcServer::new(AgentPermissions::default());

    let req = json!({
        "jsonrpc": "2.0",
        "id": 50,
        "method": "knc_eval_dual",
        "params": {
            "session_id": "no-ast"
        }
    });

    let resp_str = server.dispatch_request(&req.to_string());
    let resp: serde_json::Value = serde_json::from_str(&resp_str).expect("Valid JSON response");

    assert!(resp.get("error").is_some());
    assert_eq!(resp["error"]["code"].as_i64().unwrap(), -32602);
}
