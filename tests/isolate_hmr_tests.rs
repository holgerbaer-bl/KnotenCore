use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::{KNC_PROTOCOL_VERSION, RpcServer};
use knoten_core_types::ast::Node;
use serde_json::json;

#[test]
fn test_version_assertion_sprint337() {
    assert_eq!(KNC_PROTOCOL_VERSION, "v2.24.14");
}

#[test]
fn test_hmr_preserves_session_variables() {
    let server = RpcServer::new(AgentPermissions::default());
    server.set_revoked_keys_path(None);

    let session_id = "hmr-session-1";

    // 1. Initial AST: Assign x = 42
    let ast1 = Node::Block(vec![Node::Assign(
        "x".to_string(),
        Box::new(Node::IntLiteral(42)),
    )]);

    let req_exec1 = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_execute",
        "params": {
            "session_id": session_id,
            "ast": serde_json::to_value(&ast1).unwrap()
        }
    });

    let resp1_str = server.dispatch_request(&req_exec1.to_string());
    let resp1: serde_json::Value = serde_json::from_str(&resp1_str).unwrap();
    assert_eq!(resp1["result"]["status"], "ok");

    // 2. Perform HMR with new AST: x + 10
    let ast2 = Node::Add(
        Box::new(Node::Identifier("x".to_string())),
        Box::new(Node::IntLiteral(10)),
    );

    let req_reload = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "knc_isolate_reload",
        "params": {
            "session_id": session_id,
            "ast": serde_json::to_value(&ast2).unwrap()
        }
    });

    let resp_reload_str = server.dispatch_request(&req_reload.to_string());
    let resp_reload: serde_json::Value = serde_json::from_str(&resp_reload_str).unwrap();
    assert_eq!(resp_reload["result"]["status"], "ok");
    assert_eq!(resp_reload["result"]["report"]["reloaded"], true);
    assert_eq!(resp_reload["result"]["report"]["preserved_variables"], 1);

    // 3. Execute reloaded code — should use preserved variable x = 42 and return 52
    let req_exec2 = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "knc_execute",
        "params": {
            "session_id": session_id
        }
    });

    let resp2_str = server.dispatch_request(&req_exec2.to_string());
    let resp2: serde_json::Value = serde_json::from_str(&resp2_str).unwrap();
    assert_eq!(resp2["result"]["status"], "ok");
    assert_eq!(resp2["result"]["result"]["Int"], 52);
}

#[test]
fn test_hmr_invalid_ast_rolls_back() {
    let server = RpcServer::new(AgentPermissions::default());
    server.set_revoked_keys_path(None);

    let session_id = "hmr-session-rollback";

    // 1. Initial valid AST
    let ast1 = Node::IntLiteral(100);
    let req_exec1 = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_execute",
        "params": {
            "session_id": session_id,
            "ast": serde_json::to_value(&ast1).unwrap()
        }
    });
    let _ = server.dispatch_request(&req_exec1.to_string());

    // 2. Attempt HMR with invalid AST JSON payload
    let req_reload_invalid = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "knc_isolate_reload",
        "params": {
            "session_id": session_id,
            "ast": { "invalid_node_kind": 9999 }
        }
    });

    let resp_invalid_str = server.dispatch_request(&req_reload_invalid.to_string());
    let resp_invalid: serde_json::Value = serde_json::from_str(&resp_invalid_str).unwrap();
    assert_eq!(resp_invalid["error"]["code"], -32602);

    // 3. Original code execution remains intact
    let req_exec2 = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "knc_execute",
        "params": {
            "session_id": session_id
        }
    });
    let resp2_str = server.dispatch_request(&req_exec2.to_string());
    let resp2: serde_json::Value = serde_json::from_str(&resp2_str).unwrap();
    assert_eq!(resp2["result"]["status"], "ok");
    assert_eq!(resp2["result"]["result"]["Int"], 100);
}

#[test]
fn test_hmr_unauthenticated_rejected() {
    let token = "hmr-secret".to_string();
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "hmr-node",
        "127.0.0.1:0",
        Some(token),
    );
    server.set_revoked_keys_path(None);

    let req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_isolate_reload",
        "params": {
            "session_id": "s1",
            "ast": serde_json::to_value(Node::IntLiteral(1)).unwrap()
        }
    });

    let resp_str = server.dispatch_request(&req.to_string());
    let resp: serde_json::Value = serde_json::from_str(&resp_str).unwrap();
    assert_eq!(resp["error"]["code"], -32001);
}

#[test]
fn test_hmr_under_custom_quota() {
    let server = RpcServer::new(AgentPermissions::default());
    server.set_revoked_keys_path(None);

    let session_id = "hmr-quota-session";

    // 1. Initial AST execution with custom quota
    let ast1 = Node::IntLiteral(77);
    let req_exec1 = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_execute",
        "params": {
            "session_id": session_id,
            "ast": serde_json::to_value(&ast1).unwrap(),
            "quota": {
                "max_instructions": 750,
                "max_memory_bytes": 1048576,
                "execution_timeout_ms": 500
            }
        }
    });
    let _ = server.dispatch_request(&req_exec1.to_string());

    // Verify initial quota
    {
        let sessions = server.sessions.lock().unwrap();
        let session = sessions.get(session_id).unwrap();
        assert_eq!(session.vm.quota.max_instructions, 750);
        assert_eq!(session.vm.quota.max_memory_bytes, 1048576);
    }

    // 2. Perform HMR
    let ast2 = Node::IntLiteral(88);
    let req_reload = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "knc_isolate_reload",
        "params": {
            "session_id": session_id,
            "ast": serde_json::to_value(&ast2).unwrap()
        }
    });
    let resp_reload_str = server.dispatch_request(&req_reload.to_string());
    assert!(resp_reload_str.contains("\"status\":\"ok\""));

    // 3. Verify quota preserved across reload
    {
        let sessions = server.sessions.lock().unwrap();
        let session = sessions.get(session_id).unwrap();
        assert_eq!(session.vm.quota.max_instructions, 750);
        assert_eq!(session.vm.quota.max_memory_bytes, 1048576);
    }
}
