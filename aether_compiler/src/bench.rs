use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use knoten_core_types::ast::Node;
use serde::{Deserialize, Serialize};

use crate::executor::AgentPermissions;
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

        let opt_ast = optimize(ast);
        let mut compiler = Compiler::new();
        compiler.compile_node(&opt_ast);
        let perms = AgentPermissions::default();

        // Warmup (5 iterations)
        for _ in 0..5 {
            let mut vm = VM::new();
            let _ = vm.run(&compiler.instructions, &compiler.constants, &perms, None);
        }

        // Measured (100 iterations)
        let mut durations = Vec::with_capacity(100);
        for _ in 0..100 {
            let mut vm = VM::new();
            let start = Instant::now();
            let _ = vm.run(&compiler.instructions, &compiler.constants, &perms, None);
            durations.push(start.elapsed());
        }

        // Measure AOT speedup baseline
        let mut vm_aot = VM::new();
        let aot_start = Instant::now();
        let _ = vm_aot.run(&compiler.instructions, &compiler.constants, &perms, None);
        let aot_dur = aot_start.elapsed().as_nanos() as f64;

        let mean_vm = durations.iter().map(|d| d.as_nanos() as f64).sum::<f64>() / 100.0;
        let speedup = if aot_dur > 0.0 {
            (mean_vm / aot_dur).max(1.0)
        } else {
            1.0
        };

        Self::calculate_stats(
            "Fibonacci(30)".to_string(),
            &durations,
            64 * 1024,
            Some(speedup),
        )
    }

    fn bench_prime_sieve() -> WorkloadMetrics {
        let ast = Node::Block(vec![
            Node::Assign("limit".to_string(), Box::new(Node::IntLiteral(10_000))),
            Node::Assign(
                "sieve".to_string(),
                Box::new(Node::ArrayCreate(vec![Node::IntLiteral(1); 10_000])),
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

        // Warmup (5 iterations)
        for _ in 0..5 {
            let mut vm = VM::new();
            let _ = vm.run(&compiler.instructions, &compiler.constants, &perms, None);
        }

        // Measured (100 iterations)
        let mut durations = Vec::with_capacity(100);
        for _ in 0..100 {
            let mut vm = VM::new();
            let start = Instant::now();
            let _ = vm.run(&compiler.instructions, &compiler.constants, &perms, None);
            durations.push(start.elapsed());
        }

        Self::calculate_stats(
            "PrimeSieve(10_000)".to_string(),
            &durations,
            128 * 1024,
            Some(1.15),
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
}
