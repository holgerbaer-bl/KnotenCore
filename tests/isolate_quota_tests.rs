// Sprint 311: Isolate Multi-Tenant Quotas & JSON-RPC Session Enforcement Integration Tests
//
// Tests verify:
//   1. Execution instruction cap quota exceeded returns JSON-RPC error code -32000
//   2. Compile AST node quota exceeded returns JSON-RPC error code -32000
//   3. Multi-tenant session isolation: independent session state and quota boundaries
//   4. Normal execution under custom quota succeeds

use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::RpcServer;
use knoten_core_types::ast::{IsolateQuota, Node};
use serde_json::{Value, json};

fn test_perms() -> AgentPermissions {
    AgentPermissions {
        allow_network: false,
        allowed_domains: vec![],
        allow_fs_read: false,
        allow_fs_write: false,
    }
}

fn create_long_loop_ast() -> Node {
    // i = 0; while (i < 10000) { i = i + 1; }
    Node::Block(vec![
        Node::Assign("i".to_string(), Box::new(Node::IntLiteral(0))),
        Node::While(
            Box::new(Node::Lt(
                Box::new(Node::Identifier("i".to_string())),
                Box::new(Node::IntLiteral(10000)),
            )),
            Box::new(Node::Assign(
                "i".to_string(),
                Box::new(Node::Add(
                    Box::new(Node::Identifier("i".to_string())),
                    Box::new(Node::IntLiteral(1)),
                )),
            )),
        ),
    ])
}

#[test]
fn test_rpc_instruction_quota_exceeded() {
    let server = RpcServer::new(test_perms());
    let ast = create_long_loop_ast();

    // Custom quota with low max_instructions (30 opcodes)
    let quota = IsolateQuota {
        max_instructions: 30,
        max_memory_bytes: 16 * 1024 * 1024,
        execution_timeout_ms: 0,
    };

    let req = json!({
        "jsonrpc": "2.0",
        "method": "knc_execute",
        "params": {
            "session_id": "long_loop_session",
            "ast": ast,
            "quota": quota
        },
        "id": 100
    });

    let resp_str = server.dispatch_request(&req.to_string());
    let resp: Value = serde_json::from_str(&resp_str).unwrap();

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 100);
    assert!(
        resp["error"].is_object(),
        "Response must contain error object"
    );
    assert_eq!(
        resp["error"]["code"], -32000,
        "Quota error code must be -32000"
    );
    assert!(
        resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Quota Exceeded"),
        "Error message must indicate Quota Exceeded"
    );
}

#[test]
fn test_rpc_compile_ast_node_quota_exceeded() {
    let server = RpcServer::new(test_perms());

    // AST with multiple nodes
    let ast = Node::Block(vec![
        Node::Add(Box::new(Node::IntLiteral(1)), Box::new(Node::IntLiteral(2))),
        Node::Add(Box::new(Node::IntLiteral(3)), Box::new(Node::IntLiteral(4))),
    ]);

    // Quota with max_instructions = 2 (less than node count)
    let quota = IsolateQuota {
        max_instructions: 2,
        max_memory_bytes: 16 * 1024 * 1024,
        execution_timeout_ms: 0,
    };

    let req = json!({
        "jsonrpc": "2.0",
        "method": "knc_compile",
        "params": {
            "ast": ast,
            "quota": quota
        },
        "id": 101
    });

    let resp_str = server.dispatch_request(&req.to_string());
    let resp: Value = serde_json::from_str(&resp_str).unwrap();

    assert_eq!(resp["error"]["code"], -32000);
    assert!(
        resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Quota Exceeded")
    );
}

#[test]
fn test_rpc_multi_tenant_session_quota_isolation() {
    let server = RpcServer::new(test_perms());

    let long_loop_ast = create_long_loop_ast();
    let valid_ast = Node::Add(
        Box::new(Node::IntLiteral(10)),
        Box::new(Node::IntLiteral(20)),
    );

    // Tenant A: Strict low quota -> fails with -32000
    let req_a = json!({
        "jsonrpc": "2.0",
        "method": "knc_execute",
        "params": {
            "session_id": "tenant_a",
            "ast": long_loop_ast,
            "quota": { "max_instructions": 30, "max_memory_bytes": 1048576, "execution_timeout_ms": 0 }
        },
        "id": 200
    });

    let resp_a: Value = serde_json::from_str(&server.dispatch_request(&req_a.to_string())).unwrap();
    assert_eq!(resp_a["error"]["code"], -32000);

    // Tenant B: Independent session with default quota -> succeeds with 30
    let req_b = json!({
        "jsonrpc": "2.0",
        "method": "knc_execute",
        "params": {
            "session_id": "tenant_b",
            "ast": valid_ast
        },
        "id": 201
    });

    let resp_b: Value = serde_json::from_str(&server.dispatch_request(&req_b.to_string())).unwrap();
    assert_eq!(resp_b["result"]["status"], "ok");
    assert_eq!(resp_b["result"]["result"]["Int"], 30);
}

#[test]
fn test_rpc_normal_execution_under_custom_quota() {
    let server = RpcServer::new(test_perms());

    let ast = Node::Add(
        Box::new(Node::IntLiteral(50)),
        Box::new(Node::IntLiteral(50)),
    );
    let quota = IsolateQuota {
        max_instructions: 10_000,
        max_memory_bytes: 8 * 1024 * 1024,
        execution_timeout_ms: 1000,
    };

    let req = json!({
        "jsonrpc": "2.0",
        "method": "knc_execute",
        "params": {
            "session_id": "valid_session",
            "ast": ast,
            "quota": quota
        },
        "id": 300
    });

    let resp: Value = serde_json::from_str(&server.dispatch_request(&req.to_string())).unwrap();
    assert_eq!(resp["result"]["status"], "ok");
    assert_eq!(resp["result"]["result"]["Int"], 100);
}
