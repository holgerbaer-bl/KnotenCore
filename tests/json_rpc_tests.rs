// Sprint 310: Headless JSON-RPC 2.0 Server & Transport Protocol Integration Tests
//
// Tests verify:
//   1. knc_compile: AST node validation and AOT bytecode compilation over JSON-RPC 2.0
//   2. knc_execute: Script compilation, Stack-VM execution, result return & VmEvent collection
//   3. knc_yield_resume: Pausing execution via Node::Yield and resuming via JSON-RPC
//   4. knc_inspect_state: Inspecting active VM state, IP, stack size, and inspector metrics
//   5. Protocol Errors: Invalid JSON (-32700), Invalid Method (-32601), Invalid Params (-32602)

use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::RpcServer;
use knoten_core_types::ast::Node;
use serde_json::{Value, json};

fn test_perms() -> AgentPermissions {
    AgentPermissions {
        allow_network: false,
        allowed_domains: vec![],
        allow_fs_read: false,
        allow_fs_write: false,
    }
}

#[test]
fn test_json_rpc_knc_compile() {
    let server = RpcServer::new(test_perms());

    let ast = Node::Add(
        Box::new(Node::IntLiteral(10)),
        Box::new(Node::IntLiteral(20)),
    );
    let req = json!({
        "jsonrpc": "2.0",
        "method": "knc_compile",
        "params": { "ast": ast },
        "id": 1
    });

    let resp_str = server.dispatch_request(&req.to_string());
    let resp: Value = serde_json::from_str(&resp_str).unwrap();

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 1);
    assert_eq!(resp["result"]["status"], "ok");
    assert!(resp["result"]["instruction_count"].as_u64().unwrap() > 0);
    assert!(resp["result"]["instructions"].is_array());
}

#[test]
fn test_json_rpc_knc_execute_and_events() {
    let server = RpcServer::new(test_perms());

    // AST: EventEmit("topic_test", 123); Return 100 + 200
    let ast = Node::Block(vec![
        Node::EventEmit(
            Box::new(Node::StringLiteral("topic_test".to_string())),
            Box::new(Node::IntLiteral(123)),
        ),
        Node::Add(
            Box::new(Node::IntLiteral(100)),
            Box::new(Node::IntLiteral(200)),
        ),
    ]);

    let req = json!({
        "jsonrpc": "2.0",
        "method": "knc_execute",
        "params": {
            "session_id": "exec_test",
            "ast": ast
        },
        "id": 2
    });

    let resp_str = server.dispatch_request(&req.to_string());
    let resp: Value = serde_json::from_str(&resp_str).unwrap();

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 2);
    assert_eq!(resp["result"]["status"], "ok");
    assert_eq!(resp["result"]["session_id"], "exec_test");
    assert_eq!(resp["result"]["result"]["Int"], 300);
    assert_eq!(resp["result"]["is_yielded"], false);

    // Verify VmEvent emission returned over RPC
    let events = resp["result"]["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["Custom"]["topic"], "topic_test");
}

#[test]
fn test_json_rpc_knc_yield_resume() {
    let server = RpcServer::new(test_perms());

    // AST: Assign("val", 42); Yield; Return "val"
    let ast = Node::Block(vec![
        Node::Assign("val".to_string(), Box::new(Node::IntLiteral(42))),
        Node::Yield,
        Node::Identifier("val".to_string()),
    ]);

    // 1. Initial Execute -> Hits Yield
    let exec_req = json!({
        "jsonrpc": "2.0",
        "method": "knc_execute",
        "params": {
            "session_id": "yield_session",
            "ast": ast
        },
        "id": 10
    });

    let resp1: Value =
        serde_json::from_str(&server.dispatch_request(&exec_req.to_string())).unwrap();
    assert_eq!(resp1["result"]["status"], "ok");
    assert_eq!(resp1["result"]["is_yielded"], true);
    assert!(
        resp1["result"]["execution_state"]
            .as_str()
            .unwrap()
            .contains("Yielded")
    );

    // 2. Resume Session
    let resume_req = json!({
        "jsonrpc": "2.0",
        "method": "knc_yield_resume",
        "params": { "session_id": "yield_session" },
        "id": 11
    });

    let resp2: Value =
        serde_json::from_str(&server.dispatch_request(&resume_req.to_string())).unwrap();
    assert_eq!(resp2["result"]["status"], "ok");
    assert_eq!(resp2["result"]["is_yielded"], false);
    assert_eq!(resp2["result"]["result"]["Int"], 42);
}

#[test]
fn test_json_rpc_knc_inspect_state() {
    let server = RpcServer::new(test_perms());

    // Execute script with Yield to keep session active
    let ast = Node::Block(vec![
        Node::Assign("counter".to_string(), Box::new(Node::IntLiteral(777))),
        Node::Yield,
    ]);

    let exec_req = json!({
        "jsonrpc": "2.0",
        "method": "knc_execute",
        "params": {
            "session_id": "inspect_session",
            "ast": ast
        },
        "id": 20
    });
    server.dispatch_request(&exec_req.to_string());

    // Inspect Session
    let inspect_req = json!({
        "jsonrpc": "2.0",
        "method": "knc_inspect_state",
        "params": { "session_id": "inspect_session" },
        "id": 21
    });

    let resp: Value =
        serde_json::from_str(&server.dispatch_request(&inspect_req.to_string())).unwrap();
    assert_eq!(resp["result"]["status"], "ok");
    assert_eq!(resp["result"]["session_id"], "inspect_session");
    assert_eq!(resp["result"]["is_yielded"], true);
    assert_eq!(resp["result"]["globals_count"], 1);
    assert!(resp["result"]["inspector"].is_object());
}

#[test]
fn test_json_rpc_error_handling() {
    let server = RpcServer::new(test_perms());

    // 1. Invalid JSON
    let resp1: Value = serde_json::from_str(&server.dispatch_request("invalid { json")).unwrap();
    assert_eq!(resp1["error"]["code"], -32700);

    // 2. Invalid Protocol Version
    let req2 = json!({ "jsonrpc": "1.0", "method": "knc_compile", "id": 1 });
    let resp2: Value = serde_json::from_str(&server.dispatch_request(&req2.to_string())).unwrap();
    assert_eq!(resp2["error"]["code"], -32600);

    // 3. Unknown Method
    let req3 = json!({ "jsonrpc": "2.0", "method": "unknown_method", "id": 2 });
    let resp3: Value = serde_json::from_str(&server.dispatch_request(&req3.to_string())).unwrap();
    assert_eq!(resp3["error"]["code"], -32601);

    // 4. Invalid Params
    let req4 = json!({ "jsonrpc": "2.0", "method": "knc_compile", "params": {}, "id": 3 });
    let resp4: Value = serde_json::from_str(&server.dispatch_request(&req4.to_string())).unwrap();
    assert_eq!(resp4["error"]["code"], -32602);
}
