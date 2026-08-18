use aether_compiler::executor::{AgentPermissions, ExecResult, ExecutionEngine};
use aether_compiler::optimizer::optimize;
use aether_compiler::rpc::{KNC_PROTOCOL_VERSION, RpcServer};
use aether_compiler::vm::compiler::Compiler;
use aether_compiler::vm::machine::{VM, VMError};
use knoten_core_types::ast::Node;

#[test]
fn test_version_assertion_sprint344() {
    assert_eq!(KNC_PROTOCOL_VERSION, "v2.24.6");
    let server = RpcServer::new(AgentPermissions::default());
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_mesh_metrics",
        "params": {}
    });
    let resp = server.dispatch_request(&req.to_string());
    assert!(resp.contains("\"protocol_version\":\"v2.24.6\""));
}

#[test]
fn test_vector_dot_product_parity() {
    let size = 1_000;
    let v1_elems: Vec<Node> = (0..size)
        .map(|i| Node::FloatLiteral(1.0 + (i % 10) as f64))
        .collect();
    let v2_elems: Vec<Node> = (0..size)
        .map(|i| Node::FloatLiteral(2.0 + (i % 5) as f64))
        .collect();

    let ast = Node::VectorDot(
        Box::new(Node::ArrayCreate(v1_elems)),
        Box::new(Node::ArrayCreate(v2_elems)),
    );

    let opt_ast = optimize(ast);
    let mut compiler = Compiler::new();
    assert!(compiler.compile_node(&opt_ast));

    let mut eval_engine = ExecutionEngine::new();
    let eval_res = match eval_engine.evaluate(&opt_ast) {
        ExecResult::Value(v) | ExecResult::ReturnBlockInfo(v) => v,
        ExecResult::Fault { msg, .. } => panic!("Evaluator fault: {}", msg),
    };

    let perms = AgentPermissions::default();
    let mut vm = VM::new();
    let vm_res = vm
        .run(&compiler.instructions, &compiler.constants, &perms, None)
        .expect("VM execution failed");

    assert_eq!(
        eval_res, vm_res,
        "Evaluator and VM must yield identical dot product results"
    );
}

#[test]
fn test_vector_gas_accounting() {
    let size = 100;
    let v1_elems: Vec<Node> = (0..size).map(Node::IntLiteral).collect();
    let v2_elems: Vec<Node> = (0..size).map(Node::IntLiteral).collect();

    let ast = Node::VectorDot(
        Box::new(Node::ArrayCreate(v1_elems)),
        Box::new(Node::ArrayCreate(v2_elems)),
    );

    let opt_ast = optimize(ast);
    let mut vm = VM::new();
    // Setting max instructions lower than element count + setup should trigger GasExhausted
    let res = vm.run_with_quota(&opt_ast, 50, 16 * 1024 * 1024);
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
