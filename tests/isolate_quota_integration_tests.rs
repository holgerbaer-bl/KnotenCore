use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::{KNC_PROTOCOL_VERSION, RpcServer};
use aether_compiler::vm::machine::{VMError, VM};
use knoten_core_types::ast::Node;

#[test]
fn test_version_assertion_sprint343() {
    assert_eq!(KNC_PROTOCOL_VERSION, "v2.24.4");
    let server = RpcServer::new(AgentPermissions::default());
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_mesh_metrics",
        "params": {}
    });
    let resp = server.dispatch_request(&req.to_string());
    assert!(resp.contains("\"protocol_version\":\"v2.24.4\""));
}

#[test]
fn test_gas_exhaustion_deterministic_abort() {
    // Dynamic infinite loop node: let c = 0; while c >= 0 { c = c + 1; }
    let infinite_loop = Node::Block(vec![
        Node::Assign("c".to_string(), Box::new(Node::IntLiteral(0))),
        Node::While(
            Box::new(Node::Gte(
                Box::new(Node::Identifier("c".to_string())),
                Box::new(Node::IntLiteral(0)),
            )),
            Box::new(Node::Assign(
                "c".to_string(),
                Box::new(Node::Add(
                    Box::new(Node::Identifier("c".to_string())),
                    Box::new(Node::IntLiteral(1)),
                )),
            )),
        ),
    ]);

    let mut vm = VM::new();
    let res = vm.run_with_quota(&infinite_loop, 50, 16 * 1024 * 1024);
    assert!(res.is_err());
    let err = res.unwrap_err();
    match err {
        VMError::GasExhausted {
            executed_instructions,
            limit,
        } => {
            assert!(executed_instructions >= 50);
            assert_eq!(limit, 50);
        }
        other => panic!("Expected VMError::GasExhausted, got {:?}", other),
    }
}

#[test]
fn test_memory_quota_enforcement() {
    // Create an array with many elements: ArrayCreate with 100 elements
    let elements: Vec<Node> = (0..100).map(Node::IntLiteral).collect();
    let big_array_node = Node::ArrayCreate(elements);

    let mut vm = VM::new();
    // Set a very strict memory quota (e.g. 100 bytes)
    let res = vm.run_with_quota(&big_array_node, 100_000, 100);
    assert!(res.is_err());
    let err = res.unwrap_err();
    match err {
        VMError::MemoryQuotaExceeded {
            current_bytes,
            limit_bytes,
        } => {
            assert!(current_bytes > 100);
            assert_eq!(limit_bytes, 100);
        }
        other => panic!("Expected VMError::MemoryQuotaExceeded, got {:?}", other),
    }
}

#[test]
fn test_rpc_isolate_quota_enforcement() {
    let server = RpcServer::new(AgentPermissions::default());

    let infinite_loop = Node::Block(vec![
        Node::Assign("y".to_string(), Box::new(Node::IntLiteral(0))),
        Node::While(
            Box::new(Node::Gte(
                Box::new(Node::Identifier("y".to_string())),
                Box::new(Node::IntLiteral(0)),
            )),
            Box::new(Node::Assign(
                "y".to_string(),
                Box::new(Node::Add(
                    Box::new(Node::Identifier("y".to_string())),
                    Box::new(Node::IntLiteral(1)),
                )),
            )),
        ),
    ]);

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_eval_isolate",
        "params": {
            "ast": infinite_loop,
            "max_instructions": 30
        }
    });

    let resp = server.dispatch_request(&req.to_string());
    assert!(resp.contains("-32000"));
    assert!(resp.contains("Quota Exceeded"));
    assert!(resp.contains("GasExhausted"));
}
