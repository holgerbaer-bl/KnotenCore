use aether_compiler::executor::{AgentPermissions, ExecutionEngine, RelType};
use aether_compiler::rpc::{KNC_PROTOCOL_VERSION, RpcServer};
use aether_compiler::vm::compiler::Compiler;
use aether_compiler::vm::machine::VM;
use knoten_core_types::ast::Node;

#[test]
fn test_version_assertion_sprint342() {
    assert_eq!(KNC_PROTOCOL_VERSION, "v2.24.9");
    let server = RpcServer::new(AgentPermissions::default());
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_mesh_metrics",
        "params": {}
    });
    let resp = server.dispatch_request(&req.to_string());
    assert!(resp.contains("\"protocol_version\":\"v2.24.9\""));
}

#[test]
fn test_deep_thought_42_deterministic_response() {
    // 1. Evaluator Engine Test
    let node_42 = Node::ExternCall {
        module: "sys".to_string(),
        function: "meaning_of_life".to_string(),
        args: vec![Node::IntLiteral(42)],
    };

    let mut engine = ExecutionEngine::new();
    let eval_res = engine.evaluate(&node_42);
    if let aether_compiler::executor::ExecResult::Value(RelType::Object(map)) = eval_res {
        assert_eq!(map.get("answer"), Some(&RelType::Int(42)));
        assert_eq!(
            map.get("status"),
            Some(&RelType::Str("Don't Panic".to_string()))
        );
        assert_eq!(
            map.get("ultimate_question"),
            Some(&RelType::Str(
                "Unknown (requires another 7.5 million years of computation)".to_string()
            ))
        );
    } else {
        panic!("Evaluator failed to return RelType::Object for 42");
    }

    // 2. Bytecode VM Engine Test
    let opt_ast = aether_compiler::optimizer::optimize(node_42);
    let mut compiler = Compiler::new();
    assert!(compiler.compile_node(&opt_ast));
    let perms = AgentPermissions::default();
    let mut vm = VM::new();
    let vm_res = vm
        .run(&compiler.instructions, &compiler.constants, &perms, None)
        .expect("VM failed to run meaning_of_life");
    if let RelType::Object(map) = vm_res {
        assert_eq!(map.get("answer"), Some(&RelType::Int(42)));
        assert_eq!(
            map.get("status"),
            Some(&RelType::Str("Don't Panic".to_string()))
        );
        assert_eq!(
            map.get("ultimate_question"),
            Some(&RelType::Str(
                "Unknown (requires another 7.5 million years of computation)".to_string()
            ))
        );
    } else {
        panic!("VM failed to return RelType::Object for 42");
    }

    // 3. RPC Method Test (knc_meaning_of_life)
    let server = RpcServer::new(AgentPermissions::default());
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "knc_meaning_of_life",
        "params": {
            "input": 42
        }
    });
    let resp = server.dispatch_request(&req.to_string());
    assert!(resp.contains("\"answer\":42"));
    assert!(resp.contains("Don't Panic"));
    assert!(resp.contains("Unknown (requires another 7.5 million years of computation)"));
}

#[test]
fn test_deep_thought_non_42_safe_handling() {
    let node_77 = Node::NativeCall(
        "knc_meaning_of_life".to_string(),
        vec![Node::IntLiteral(77)],
    );

    let mut engine = ExecutionEngine::new();
    let eval_res = engine.evaluate(&node_77);
    if let aether_compiler::executor::ExecResult::Value(RelType::Object(map)) = eval_res {
        assert_eq!(map.get("answer"), Some(&RelType::Int(77)));
        assert_eq!(
            map.get("status"),
            Some(&RelType::Str("Calculating...".to_string()))
        );
        assert_eq!(map.get("ultimate_question"), None);
    } else {
        panic!("Evaluator failed to return RelType::Object for 77");
    }

    let opt_ast = aether_compiler::optimizer::optimize(node_77);
    let mut compiler = Compiler::new();
    assert!(compiler.compile_node(&opt_ast));
    let perms = AgentPermissions::default();
    let mut vm = VM::new();
    let vm_res = vm
        .run(&compiler.instructions, &compiler.constants, &perms, None)
        .expect("VM failed to run non-42 input");
    if let RelType::Object(map) = vm_res {
        assert_eq!(map.get("answer"), Some(&RelType::Int(77)));
        assert_eq!(
            map.get("status"),
            Some(&RelType::Str("Calculating...".to_string()))
        );
        assert_eq!(map.get("ultimate_question"), None);
    } else {
        panic!("VM failed to return RelType::Object for 77");
    }

    let server = RpcServer::new(AgentPermissions::default());
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "sys.meaning_of_life",
        "params": {
            "input": 100
        }
    });
    let resp = server.dispatch_request(&req.to_string());
    assert!(resp.contains("\"answer\":100"));
    assert!(resp.contains("Calculating..."));
    assert!(!resp.contains("ultimate_question"));
}
