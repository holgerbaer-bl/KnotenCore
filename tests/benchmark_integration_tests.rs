use aether_compiler::bench::{BenchmarkEngine, BenchmarkReport, WorkloadMetrics};
use aether_compiler::executor::{AgentPermissions, ExecResult, ExecutionEngine};
use aether_compiler::optimizer::optimize;
use aether_compiler::rpc::{KNC_PROTOCOL_VERSION, RpcServer};
use aether_compiler::vm::compiler::Compiler;
use aether_compiler::vm::machine::VM;
use knoten_core_types::ast::Node;

#[test]
fn test_version_assertion_sprint340() {
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
fn test_handlers_reexports_complete() {
    let server = RpcServer::new(AgentPermissions::default());
    let req_agent = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_agent_handshake",
        "params": {
            "node_id": "test-agent",
            "capabilities": ["compute"]
        }
    });
    let resp_agent = server.dispatch_request(&req_agent.to_string());
    assert!(resp_agent.contains("\"status\":\"ok\""));

    // Verify handlers re-exports from aether_compiler::rpc::handlers::* and aether_compiler::rpc::*
    let dummy_id = Some(serde_json::json!(100));
    let res_a =
        server.handle_agent_handshake(dummy_id.clone(), serde_json::json!({"node_id": "a"}));
    let res_c = server.handle_compile(dummy_id, serde_json::json!({"ast": Node::IntLiteral(1)}));
    assert!(res_a.result.is_some());
    assert!(res_c.result.is_some());
}

#[test]
fn test_benchmark_engine_direct_api() {
    let report: BenchmarkReport = BenchmarkEngine::run_all();
    assert_eq!(report.protocol_version, "v2.24.9");
    assert!(report.metrics.len() >= 5);

    for m in &report.metrics {
        assert!(m.iterations > 0);
        assert!(m.mean_ns > 0.0);
        assert!(m.ops_per_sec > 0.0);
    }

    let json_str = serde_json::to_string_pretty(&report).unwrap();
    assert!(json_str.contains("\"protocol_version\": \"v2.24.9\""));
    assert!(json_str.contains("Fibonacci(30)"));
    assert!(json_str.contains("PrimeSieve(10_000)"));
    assert!(json_str.contains("VectorDotProduct(100_000)"));

    let single: Option<WorkloadMetrics> = BenchmarkEngine::run_workload("Fibonacci(30)");
    assert!(single.is_some());
    let fib_m = single.unwrap();
    assert_eq!(fib_m.workload_name, "Fibonacci(30)");
    assert!(fib_m.aot_speedup.is_some());
}

#[test]
fn test_true_evaluator_vs_vm_benchmark_parity() {
    let fib_ast = Node::Block(vec![
        Node::Assign("n".to_string(), Box::new(Node::IntLiteral(30))),
        Node::Assign("a".to_string(), Box::new(Node::IntLiteral(0))),
        Node::Assign("b".to_string(), Box::new(Node::IntLiteral(1))),
        Node::Assign("i".to_string(), Box::new(Node::IntLiteral(0))),
        Node::While(
            Box::new(Node::Lt(
                Box::new(Node::Identifier("i".to_string())),
                Box::new(Node::Identifier("n".to_string())),
            )),
            Box::new(Node::Block(vec![
                Node::Assign(
                    "temp".to_string(),
                    Box::new(Node::Add(
                        Box::new(Node::Identifier("a".to_string())),
                        Box::new(Node::Identifier("b".to_string())),
                    )),
                ),
                Node::Assign("a".to_string(), Box::new(Node::Identifier("b".to_string()))),
                Node::Assign(
                    "b".to_string(),
                    Box::new(Node::Identifier("temp".to_string())),
                ),
                Node::Assign(
                    "i".to_string(),
                    Box::new(Node::Add(
                        Box::new(Node::Identifier("i".to_string())),
                        Box::new(Node::IntLiteral(1)),
                    )),
                ),
            ])),
        ),
        Node::Identifier("a".to_string()),
    ]);

    let opt_ast = optimize(fib_ast);

    // 1. Evaluator execution
    let mut eval_engine = ExecutionEngine::new();
    let eval_res = match eval_engine.evaluate(&opt_ast) {
        ExecResult::Value(v) | ExecResult::ReturnBlockInfo(v) => v,
        ExecResult::Fault { msg, .. } => panic!("Evaluator fault: {}", msg),
    };

    // 2. VM execution
    let mut compiler = Compiler::new();
    assert!(compiler.compile_node(&opt_ast));
    let perms = AgentPermissions::default();
    let mut vm = VM::new();
    let vm_res = vm
        .run(&compiler.instructions, &compiler.constants, &perms, None)
        .expect("VM execution failed");

    // 3. Parity assertion
    assert_eq!(eval_res, vm_res);

    // 4. BenchmarkEngine execution parity verification
    let single = BenchmarkEngine::run_workload("Fibonacci(30)").unwrap();
    assert!(single.aot_speedup.unwrap() >= 1.0);
}
