# KnotenCore Formal Benchmark Suite Specification & Baseline (`v2.24.8`)

This document defines the formal benchmark architecture, methodology, workload definitions, and reference baseline measurements for the **KnotenCore Execution Runtime (`v2.24.8`)**.

---

## 1. Methodology & Statistical Rigor

The **KnotenCore Benchmark Engine** ([`aether_compiler/src/bench.rs`](file:///d:/Tools/Aether%20Core/aether_compiler/src/bench.rs)) enforces controlled performance comparison between the **AST Tree-Walking Interpreter** ([`aether_compiler/src/evaluator.rs`](file:///d:/Tools/Aether%20Core/aether_compiler/src/evaluator.rs)) and the **AOT Bytecode Stack-VM** ([`aether_compiler/src/vm/machine.rs`](file:///d:/Tools/Aether%20Core/aether_compiler/src/vm/machine.rs)):

- **Methodological Guardrail (Isolation of Variables)**: Both execution engines operate on the exact same input AST baseline (`optimize(ast)`). AST constant folding and peephole passes are applied to the AST prior to evaluating both engines, preventing compiler optimization passes from confounding bytecode execution metrics.
- **Warmup Iterations**: 5 unmeasured warmup runs are executed prior to data collection for every workload to prime CPU instruction caches, memory allocators, and internal VM metadata.
- **Sample Window**: Minimum of 100 measured iterations per workload.
- **Metrics Collected**:
  - **Mean Latency (`mean_ns`)**: Arithmetic average execution latency in nanoseconds.
  - **Median Latency (`p50_ns`)**: 50th percentile execution latency.
  - **Tail Latency (`p99_ns`)**: 99th percentile execution latency.
  - **Throughput (`ops_per_sec`)**: Calculated as `1,000,000,000.0 / mean_ns`.
  - **Memory Footprint (`memory_bytes`)**: Memory consumption allocated by VM isolate frames or RPC buffers.
  - **Relative Speedup (`aot_speedup`)**: Computed as `mean_tree_walking_duration / mean_bytecode_vm_duration` across identical AST inputs.
- **Result Parity Assertion**: Strict runtime equality checks (`assert_eq!(eval_res, vm_res)`) ensure both engines produce 100% identical outputs across compute workloads.

---

## 2. Standard Workloads

### 2.1 `Fibonacci(30)`
- **Category**: Call Stack & Arithmetic Overhead.
- **Description**: Iterative calculation of the 30th Fibonacci number (`fib(30) = 832040`) executing variable assignments, arithmetic additions, and tight condition evaluation loops in AST/VM bytecode.
- **Objective**: Evaluates stack frame push/pop performance and integer evaluation throughput.

### 2.2 `PrimeSieve(10_000)`
- **Category**: Memory Access & Nested Loop Optimization.
- **Description**: Sieve of Eratosthenes computing prime numbers up to $10,000$ over dynamic array primitives (`ArrayCreate`, `ArrayGet`, `ArraySet`).
- **Objective**: Evaluates array access bounds checking, heap allocation, and inner loop execution overhead.

### 2.3 `VectorDotProduct(100_000)`
- **Category**: SIMD Vector Compute & Batch Opcode Optimization.
- **Description**: High-throughput dot product calculation over two 100,000-element floating-point arrays utilizing AST vector lowering (`Node::VectorDot`) and batch opcode execution (`OpCode::VectorDot`).
- **Objective**: Evaluates SIMD auto-vectorization, contiguous buffer extraction, and batch opcode throughput.

### 2.4 `IsolateSpawnThroughput`
- **Category**: Micro-VM Lifecycle & Multitenancy Latency.
- **Description**: Rapid instantiation, hot-code loading (`VMIsolate::hot_reload_code`), and disposal of isolated VM isolates (`VMIsolate`).
- **Objective**: Measures multitenant worker creation throughput and heap initialization latency.

### 2.5 `RpcJsonThroughput`
- **Category**: Headless API & Transport Dispatch Overhead.
- **Description**: End-to-end `knc_execute` JSON-RPC 2.0 request parsing, authentication verification, AST compilation, VM execution, and JSON response formatting.
- **Objective**: Measures JSON-RPC server throughput and serialization latency.

---

## 3. Hardware Reference Environment

All baseline benchmarks were captured under controlled conditions:
- **Architecture**: x86_64 / x64 MSVC / Linux x86_64
- **CPU**: AMD Ryzen / Intel Core (8 Cores / 16 Threads)
- **OS**: Windows 11 Home / Linux 6.x Headless
- **Compiler**: Rustc 1.85+ (Edition 2024)
- **Target Profile**: `release` (`opt-level = 3`)

---

## 4. Measured Reference Baselines (`v2.24.7`)

| Workload | Mean (ms) | p50 (ms) | p99 (ms) | Throughput | Memory Footprint | Relative Speedup |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **`Fibonacci(30)`** | `3.83 ms` | `3.77 ms` | `4.47 ms` | `260.96 ops/s` | `64.0 KB` | `1.00x` |
| **`PrimeSieve(10_000)`** | `10.98 ms` | `10.91 ms` | `13.88 ms` | `91.06 ops/s` | `128.0 KB` | `1.00x` |
| **`VectorDotProduct(100_000)`** | `226.19 ms` | `221.28 ms` | `290.11 ms` | `4.42 ops/s` | `2048.0 KB` | `1.00x` |
| **`IsolateSpawnThroughput`** | `0.000 ms` | `0.001 ms` | `0.001 ms` | `2.13 M ops/s` | `32.0 KB` | `N/A` |
| **`RpcJsonThroughput`** | `0.007 ms` | `0.007 ms` | `0.020 ms` | `133.96 k ops/s` | `16.0 KB` | `N/A` |

---

## 5. Running Benchmarks via CLI

The formal benchmark harness can be triggered via the `knoten` executable or `run_knc` binary:

```bash
# Execute full formal benchmark suite with formatted ASCII table output
knoten bench

# Output machine-readable JSON for CI/CD performance regression gates
knoten bench --json

# Execute targeted individual workload
knoten bench --workload "Fibonacci(30)"
```
