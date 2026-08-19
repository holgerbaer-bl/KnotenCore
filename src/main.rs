use knoten_core::bench::BenchmarkEngine;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 2 && args[1] == "bench" {
        let is_json = args.iter().any(|arg| arg == "--json");
        let workload = args
            .iter()
            .position(|arg| arg == "--workload")
            .and_then(|idx| args.get(idx + 1).cloned());

        run_benchmark_cli(is_json, workload.as_deref());
        return;
    }

    println!("KnotenCore 🦀🤖 (v2.24.9) — Headless-First Deterministic Execution Runtime");
    println!("Usage:");
    println!("  knoten bench [--json] [--workload <NAME>]");
}

fn run_benchmark_cli(is_json: bool, workload: Option<&str>) {
    let report = if let Some(name) = workload {
        let metrics = match BenchmarkEngine::run_workload(name) {
            Some(m) => vec![m],
            None => {
                eprintln!(
                    "[Error] Unknown workload name '{}'. Available: Fibonacci(30), PrimeSieve(10_000), VectorDotProduct(100_000), IsolateSpawnThroughput, RpcJsonThroughput",
                    name
                );
                std::process::exit(1);
            }
        };
        knoten_core::bench::BenchmarkReport {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            protocol_version: aether_compiler::rpc::KNC_PROTOCOL_VERSION.to_string(),
            metrics,
        }
    } else {
        BenchmarkEngine::run_all()
    };

    if is_json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
        return;
    }

    println!(
        "========================================================================================="
    );
    println!(
        "                           KNOTENCORE FORMAL BENCHMARK SUITE                             "
    );
    println!(
        "                           Protocol Version: {}                                   ",
        report.protocol_version
    );
    println!(
        "========================================================================================="
    );
    println!(
        "{:<24} | {:<10} | {:<10} | {:<10} | {:<14} | {:<12} | {:<8}",
        "Workload", "Mean (ms)", "p50 (ms)", "p99 (ms)", "Throughput", "Memory", "Speedup"
    );
    println!(
        "-----------------------------------------------------------------------------------------"
    );

    for m in &report.metrics {
        let mean_ms = m.mean_ns / 1_000_000.0;
        let p50_ms = m.p50_ns / 1_000_000.0;
        let p99_ms = m.p99_ns / 1_000_000.0;
        let ops_fmt = if m.ops_per_sec >= 1_000_000.0 {
            format!("{:.2} M ops/s", m.ops_per_sec / 1_000_000.0)
        } else if m.ops_per_sec >= 1_000.0 {
            format!("{:.2} k ops/s", m.ops_per_sec / 1_000.0)
        } else {
            format!("{:.2} ops/s", m.ops_per_sec)
        };
        let mem_fmt = format!("{:.1} KB", m.memory_bytes as f64 / 1024.0);
        let speedup_fmt = m
            .aot_speedup
            .map_or("N/A".to_string(), |s| format!("{:.2}x", s));

        println!(
            "{:<24} | {:<10.3} | {:<10.3} | {:<10.3} | {:<14} | {:<12} | {:<8}",
            m.workload_name, mean_ms, p50_ms, p99_ms, ops_fmt, mem_fmt, speedup_fmt
        );
    }
    println!(
        "========================================================================================="
    );
}
