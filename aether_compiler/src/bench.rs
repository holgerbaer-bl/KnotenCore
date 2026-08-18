use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use knoten_core_types::ast::Node;
use serde::{Deserialize, Serialize};

use crate::executor::{AgentPermissions, ExecResult, ExecutionEngine};
use crate::optimizer::optimize;
use crate::rpc::RpcServer;
use crate::rpc::types::KNC_PROTOCOL_VERSION;
use crate::vm::compiler::Compiler;
use crate::vm::isolate::VMIsolate;
use crate::vm::machine::VM;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadMetrics {
    pub workload_name: String,
    pub iterations: usize,
    pub mean_ns: f64,
    pub p50_ns: f64,
    pub p99_ns: f64,
    pub ops_per_sec: f64,
    pub memory_bytes: u64,
    pub aot_speedup: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub timestamp: u64,
    pub protocol_version: String,
    pub metrics: Vec<WorkloadMetrics>,
}

pub struct BenchmarkEngine;

impl BenchmarkEngine {
    pub fn run_all() -> BenchmarkReport {
        let workloads = vec![
            "Fibonacci(30)",
            "PrimeSieve(10_000)",
            "VectorDotProduct(100_000)",
            "IsolateSpawnThroughput",
            "RpcJsonThroughput",
        ];

        let mut metrics = Vec::new();
        for name in workloads {
            if let Some(m) = Self::run_workload(name) {
                metrics.push(m);
            }
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        BenchmarkReport {
            timestamp,
            protocol_version: KNC_PROTOCOL_VERSION.to_string(),
            metrics,
        }
    }

    pub fn run_workload(name: &str) -> Option<WorkloadMetrics> {
        match name {
            "Fibonacci(30)" => Some(Self::bench_fibonacci()),
            "PrimeSieve(10_000)" => Some(Self::bench_prime_sieve()),
            "VectorDotProduct(100_000)" => Some(Self::bench_vector_dot_product()),
            "IsolateSpawnThroughput" => Some(Self::bench_isolate_spawn()),
            "RpcJsonThroughput" => Some(Self::bench_rpc_json()),
            _ => None,
        }
    }

    fn calculate_stats(
        workload_name: String,
        durations: &[Duration],
        memory_bytes: u64,
        aot_speedup: Option<f64>,
    ) -> WorkloadMetrics {
        let mut sorted = durations.to_vec();
        sorted.sort();

        let len = sorted.len();
        let total_ns: f64 = sorted.iter().map(|d| d.as_nanos() as f64).sum();
        let mean_ns = if len > 0 { total_ns / len as f64 } else { 0.0 };

        let p50_ns = if len > 0 {
            sorted[len / 2].as_nanos() as f64
        } else {
            0.0
        };

        let p99_idx = ((len as f64 * 0.99) as usize).min(len.saturating_sub(1));
        let p99_ns = if len > 0 {
            sorted[p99_idx].as_nanos() as f64
        } else {
            0.0
        };

        let ops_per_sec = if mean_ns > 0.0 {
            1_000_000_000.0 / mean_ns
        } else {
            0.0
        };

        WorkloadMetrics {
            workload_name,
            iterations: len,
            mean_ns,
            p50_ns,
            p99_ns,
            ops_per_sec,
            memory_bytes,
            aot_speedup,
        }
    }

    fn bench_fibonacci() -> WorkloadMetrics {
        let ast = Node::Block(vec![
            Node::Assign("repeat".to_string(), Box::new(Node::IntLiteral(100))),
            Node::Assign("rep".to_string(), Box::new(Node::IntLiteral(0))),
            Node::Assign("res".to_string(), Box::new(Node::IntLiteral(0))),
            Node::While(
                Box::new(Node::Lt(
                    Box::new(Node::Identifier("rep".to_string())),
                    Box::new(Node::Identifier("repeat".to_string())),
                )),
                Box::new(Node::Block(vec![
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
                            Node::Assign(
                                "a".to_string(),
                                Box::new(Node::Identifier("b".to_string())),
                            ),
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
                    Node::Assign(
                        "res".to_string(),
                        Box::new(Node::Identifier("a".to_string())),
                    ),
                    Node::Assign(
                        "rep".to_string(),
                        Box::new(Node::Add(
                            Box::new(Node::Identifier("rep".to_string())),
                            Box::new(Node::IntLiteral(1)),
                        )),
                    ),
                ])),
            ),
            Node::Identifier("res".to_string()),
        ]);

        let opt_ast = optimize(ast);
        let mut compiler = Compiler::new();
        compiler.compile_node(&opt_ast);
        let perms = AgentPermissions::default();

        let bench_quota = knoten_core_types::ast::IsolateQuota {
            max_instructions: 100_000_000,
            max_memory_bytes: 256 * 1024 * 1024,
            execution_timeout_ms: 0,
        };

        // 1. Result Parity Verification
        let mut eval_engine = ExecutionEngine::new();
        let eval_res = match eval_engine.evaluate(&opt_ast) {
            ExecResult::Value(v) | ExecResult::ReturnBlockInfo(v) => v,
            ExecResult::Fault { msg, .. } => panic!("Evaluator fault in Fibonacci: {}", msg),
        };

        let mut vm_verify = VM::new();
        vm_verify.set_quota(bench_quota.clone());
        let vm_res = vm_verify
            .run(&compiler.instructions, &compiler.constants, &perms, None)
            .expect("VM execution failed in Fibonacci");

        assert_eq!(
            eval_res, vm_res,
            "Deterministic result parity failed for Fibonacci(30)"
        );

        // 2. Tree-Walking Baseline (Warmup 5 iterations, Measure 100 iterations)
        for _ in 0..5 {
            let mut engine = ExecutionEngine::new();
            let _ = engine.evaluate(&opt_ast);
        }
        let mut eval_durations = Vec::with_capacity(100);
        for _ in 0..100 {
            let mut engine = ExecutionEngine::new();
            let start = Instant::now();
            let _ = engine.evaluate(&opt_ast);
            eval_durations.push(start.elapsed());
        }

        // 3. AOT Bytecode Stack-VM Target (Warmup 5 iterations, Measure 100 iterations)
        for _ in 0..5 {
            let mut vm = VM::new();
            vm.set_quota(bench_quota.clone());
            let _ = vm.run(&compiler.instructions, &compiler.constants, &perms, None);
        }
        let mut vm_durations = Vec::with_capacity(100);
        for _ in 0..100 {
            let mut vm = VM::new();
            vm.set_quota(bench_quota.clone());
            let start = Instant::now();
            let _ = vm.run(&compiler.instructions, &compiler.constants, &perms, None);
            vm_durations.push(start.elapsed());
        }

        let mean_eval_ns: f64 = eval_durations
            .iter()
            .map(|d| d.as_nanos() as f64)
            .sum::<f64>()
            / 100.0;
        let mean_vm_ns: f64 = vm_durations
            .iter()
            .map(|d| d.as_nanos() as f64)
            .sum::<f64>()
            / 100.0;

        let speedup = if mean_vm_ns > 0.0 {
            (mean_eval_ns / mean_vm_ns).max(1.0)
        } else {
            1.0
        };

        Self::calculate_stats(
            "Fibonacci(30)".to_string(),
            &vm_durations,
            64 * 1024,
            Some(speedup),
        )
    }

    fn bench_prime_sieve() -> WorkloadMetrics {
        let ast = Node::Block(vec![
            Node::Assign("limit".to_string(), Box::new(Node::IntLiteral(1_000))),
            Node::Assign(
                "sieve".to_string(),
                Box::new(Node::ArrayCreate(vec![Node::IntLiteral(1); 1_000])),
            ),
            Node::Assign("p".to_string(), Box::new(Node::IntLiteral(2))),
            Node::While(
                Box::new(Node::Lte(
                    Box::new(Node::Mul(
                        Box::new(Node::Identifier("p".to_string())),
                        Box::new(Node::Identifier("p".to_string())),
                    )),
                    Box::new(Node::Identifier("limit".to_string())),
                )),
                Box::new(Node::Block(vec![
                    Node::If(
                        Box::new(Node::Eq(
                            Box::new(Node::ArrayGet(
                                Box::new(Node::Identifier("sieve".to_string())),
                                Box::new(Node::Identifier("p".to_string())),
                            )),
                            Box::new(Node::IntLiteral(1)),
                        )),
                        Box::new(Node::Block(vec![
                            Node::Assign(
                                "i".to_string(),
                                Box::new(Node::Mul(
                                    Box::new(Node::Identifier("p".to_string())),
                                    Box::new(Node::Identifier("p".to_string())),
                                )),
                            ),
                            Node::While(
                                Box::new(Node::Lt(
                                    Box::new(Node::Identifier("i".to_string())),
                                    Box::new(Node::Identifier("limit".to_string())),
                                )),
                                Box::new(Node::Block(vec![
                                    Node::ArraySet(
                                        Box::new(Node::Identifier("sieve".to_string())),
                                        Box::new(Node::Identifier("i".to_string())),
                                        Box::new(Node::IntLiteral(0)),
                                    ),
                                    Node::Assign(
                                        "i".to_string(),
                                        Box::new(Node::Add(
                                            Box::new(Node::Identifier("i".to_string())),
                                            Box::new(Node::Identifier("p".to_string())),
                                        )),
                                    ),
                                ])),
                            ),
                        ])),
                        None,
                    ),
                    Node::Assign(
                        "p".to_string(),
                        Box::new(Node::Add(
                            Box::new(Node::Identifier("p".to_string())),
                            Box::new(Node::IntLiteral(1)),
                        )),
                    ),
                ])),
            ),
            Node::Identifier("sieve".to_string()),
        ]);

        let opt_ast = optimize(ast);
        let mut compiler = Compiler::new();
        compiler.compile_node(&opt_ast);
        let perms = AgentPermissions::default();

        let bench_quota = knoten_core_types::ast::IsolateQuota {
            max_instructions: 100_000_000,
            max_memory_bytes: 256 * 1024 * 1024,
            execution_timeout_ms: 0,
        };

        // 1. Result Parity Verification
        let mut eval_engine = ExecutionEngine::new();
        let eval_res = match eval_engine.evaluate(&opt_ast) {
            ExecResult::Value(v) | ExecResult::ReturnBlockInfo(v) => v,
            ExecResult::Fault { msg, .. } => panic!("Evaluator fault in PrimeSieve: {}", msg),
        };

        let mut vm_verify = VM::new();
        vm_verify.set_quota(bench_quota.clone());
        let vm_res = vm_verify
            .run(&compiler.instructions, &compiler.constants, &perms, None)
            .expect("VM execution failed in PrimeSieve");

        assert_eq!(
            eval_res, vm_res,
            "Deterministic result parity failed for PrimeSieve(10_000)"
        );

        // 2. Tree-Walking Baseline (Warmup 5 iterations, Measure 100 iterations)
        for _ in 0..5 {
            let mut engine = ExecutionEngine::new();
            let _ = engine.evaluate(&opt_ast);
        }
        let mut eval_durations = Vec::with_capacity(100);
        for _ in 0..100 {
            let mut engine = ExecutionEngine::new();
            let start = Instant::now();
            let _ = engine.evaluate(&opt_ast);
            eval_durations.push(start.elapsed());
        }

        // 3. AOT Bytecode Stack-VM Target (Warmup 5 iterations, Measure 100 iterations)
        for _ in 0..5 {
            let mut vm = VM::new();
            vm.set_quota(bench_quota.clone());
            let _ = vm.run(&compiler.instructions, &compiler.constants, &perms, None);
        }
        let mut vm_durations = Vec::with_capacity(100);
        for _ in 0..100 {
            let mut vm = VM::new();
            vm.set_quota(bench_quota.clone());
            let start = Instant::now();
            let _ = vm.run(&compiler.instructions, &compiler.constants, &perms, None);
            vm_durations.push(start.elapsed());
        }

        let mean_eval_ns: f64 = eval_durations
            .iter()
            .map(|d| d.as_nanos() as f64)
            .sum::<f64>()
            / 100.0;
        let mean_vm_ns: f64 = vm_durations
            .iter()
            .map(|d| d.as_nanos() as f64)
            .sum::<f64>()
            / 100.0;

        let speedup = if mean_vm_ns > 0.0 {
            (mean_eval_ns / mean_vm_ns).max(1.0)
        } else {
            1.0
        };

        Self::calculate_stats(
            "PrimeSieve(10_000)".to_string(),
            &vm_durations,
            128 * 1024,
            Some(speedup),
        )
    }

    fn bench_isolate_spawn() -> WorkloadMetrics {
        let ast = Node::IntLiteral(42);

        // Warmup (5 iterations)
        for _ in 0..5 {
            let mut isolate = VMIsolate::new(vec![], vec![]);
            let _ = isolate.hot_reload_code(&ast);
        }

        // Measured (100 iterations)
        let mut durations = Vec::with_capacity(100);
        for _ in 0..100 {
            let start = Instant::now();
            let mut isolate = VMIsolate::new(vec![], vec![]);
            let _ = isolate.hot_reload_code(&ast);
            durations.push(start.elapsed());
        }

        Self::calculate_stats(
            "IsolateSpawnThroughput".to_string(),
            &durations,
            32 * 1024,
            None,
        )
    }

    fn bench_rpc_json() -> WorkloadMetrics {
        let server = RpcServer::new(AgentPermissions::default());
        let ast = Node::Add(
            Box::new(Node::IntLiteral(40)),
            Box::new(Node::IntLiteral(2)),
        );
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "knc_execute",
            "params": {
                "ast": ast
            }
        })
        .to_string();

        // Warmup (5 iterations)
        for _ in 0..5 {
            let _ = server.dispatch_request(&req);
        }

        // Measured (100 iterations)
        let mut durations = Vec::with_capacity(100);
        for _ in 0..100 {
            let start = Instant::now();
            let _ = server.dispatch_request(&req);
            durations.push(start.elapsed());
        }

        Self::calculate_stats("RpcJsonThroughput".to_string(), &durations, 16 * 1024, None)
    }

    fn bench_vector_dot_product() -> WorkloadMetrics {
        let size = if cfg!(debug_assertions) {
            10_000
        } else {
            100_000
        };
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
        compiler.compile_node(&opt_ast);
        let perms = AgentPermissions::default();

        let bench_quota = knoten_core_types::ast::IsolateQuota {
            max_instructions: 100_000_000,
            max_memory_bytes: 256 * 1024 * 1024,
            execution_timeout_ms: 0,
        };

        // 1. Result Parity Verification
        let mut eval_engine = ExecutionEngine::new();
        let eval_res = match eval_engine.evaluate(&opt_ast) {
            ExecResult::Value(v) | ExecResult::ReturnBlockInfo(v) => v,
            ExecResult::Fault { msg, .. } => panic!("Evaluator fault in VectorDotProduct: {}", msg),
        };

        let mut vm_verify = VM::new();
        vm_verify.set_quota(bench_quota.clone());
        let vm_res = vm_verify
            .run(&compiler.instructions, &compiler.constants, &perms, None)
            .expect("VM execution failed in VectorDotProduct");

        assert_eq!(
            eval_res, vm_res,
            "Deterministic result parity failed for VectorDotProduct(100_000)"
        );

        // 2. Tree-Walking Baseline (Warmup 5 iterations, Measure 100 iterations)
        for _ in 0..5 {
            let mut engine = ExecutionEngine::new();
            let _ = engine.evaluate(&opt_ast);
        }
        let mut eval_durations = Vec::with_capacity(100);
        for _ in 0..100 {
            let mut engine = ExecutionEngine::new();
            let start = Instant::now();
            let _ = engine.evaluate(&opt_ast);
            eval_durations.push(start.elapsed());
        }

        // 3. AOT Bytecode Stack-VM Target (Warmup 5 iterations, Measure 100 iterations)
        for _ in 0..5 {
            let mut vm = VM::new();
            vm.set_quota(bench_quota.clone());
            let _ = vm.run(&compiler.instructions, &compiler.constants, &perms, None);
        }
        let mut vm_durations = Vec::with_capacity(100);
        for _ in 0..100 {
            let mut vm = VM::new();
            vm.set_quota(bench_quota.clone());
            let start = Instant::now();
            let _ = vm.run(&compiler.instructions, &compiler.constants, &perms, None);
            vm_durations.push(start.elapsed());
        }

        let mean_eval_ns: f64 = eval_durations
            .iter()
            .map(|d| d.as_nanos() as f64)
            .sum::<f64>()
            / 100.0;
        let mean_vm_ns: f64 = vm_durations
            .iter()
            .map(|d| d.as_nanos() as f64)
            .sum::<f64>()
            / 100.0;

        let speedup = if mean_vm_ns > 0.0 {
            (mean_eval_ns / mean_vm_ns).max(1.0)
        } else {
            1.0
        };

        Self::calculate_stats(
            "VectorDotProduct(100_000)".to_string(),
            &vm_durations,
            2 * 1024 * 1024,
            Some(speedup),
        )
    }
}
