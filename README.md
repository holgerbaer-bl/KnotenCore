# KnotenCore 🦀🤖

[![Version](https://img.shields.io/badge/version-v2.24.24-blue)](https://github.com/holgerbaer-bl/KnotenCore/releases/latest)
[![CI Quality Gates](https://github.com/holgerbaer-bl/KnotenCore/actions/workflows/ci.yml/badge.svg)](https://github.com/holgerbaer-bl/KnotenCore/actions/workflows/ci.yml)
[![AI Directives](https://img.shields.io/badge/AI--Directives-AI.md-purple)](AI.md)
[![Automated CI](https://img.shields.io/badge/Automated_CI-Active-brightgreen)](docs/workflows/agent-ci-feedback.yml)
[![Tests](https://img.shields.io/badge/tests-342%2F342-brightgreen)](https://github.com/holgerbaer-bl/KnotenCore/actions)
[![Release](https://img.shields.io/badge/release-v2.24.24-brightgreen)](https://github.com/holgerbaer-bl/KnotenCore/releases/latest)

*(Noun) /knoːtən kɔːr/*

1. **Not** a relentless underground German hardcore techno subgenre. 
2. A high-performance, headless Rust runtime & P2P mesh engine for autonomous AI agents — fully driven by JSON-AST.

**KnotenCore is a deterministic, sandboxed runtime for autonomous agents, with cryptographically authenticated distributed execution.**

## What is KnotenCore?
**KnotenCore** is a high-performance, headless Rust runtime & P2P mesh engine for autonomous AI agents — fully driven by JSON-AST. By executing structured JSON-AST nodes (`.nod` and `.knoten` files) instead of raw text, KnotenCore eliminates LLM syntax hallucinations and parser ambiguities. The engine compiles ASTs directly into an AOT-optimized bytecode stream executed by a bare-metal Register Stack-VM, or validates execution under dual-engine parity against the reference AST Tree-Walker.

### Core Capabilities:
- **Deterministic Execution & Dual-Engine Parity**: Dual-execution harness (`DualEngineValidator`) running both an AOT Stack-VM and an AST Tree-Walker interpreter over identical AST trees with IEEE-754 NaN-aware equality checks and immediate divergence quarantine containment (`ERR_ENGINE_DISCREPANCY` / `-32020`).
- **Hermetic Sandboxed Isolates**: Isolated execution environments (`VMIsolate`) enforcing strict opcode gas limits, microsecond wall-clock execution watchdog timeouts, sandboxed file I/O canonicalization, and hard heap allocation boundaries.
- **Zero-Trust Mesh & Cryptographic Identity**: Deterministic canonical self-certifying node identities (`knc-<64_hex_chars>`) derived from 32-byte Ed25519 public keys, canonical envelope signature verification, 30s replay window defense, strict legacy-HMAC isolation, and anti-downgrade enforcement.
- **Distributed CRDT State Store**: Decentralized Last-Write-Wins (LWW) conflict-free replicated data store with cryptographic SHA-256 anti-entropy state digests (`knc_store_digest`) and targeted timestamp-bounded delta synchronization (`knc_store_diff`).
- **Swarm Governance & Raft Consensus**: Distributed cluster coordination with dynamic node roles (`Leader`, `Worker`, `Storage`, `Observer`), term transitions, quorum-based voting (`RequestVote`), background heartbeats, and fail-closed lock poisoning resilience.
- **Bare-Metal Performance & SIMD**: 1.21x out-of-the-box native AOT speedup over tree-walking evaluation, contiguous vector buffer representations (`Vec<f64>`, `Vec<i64>`), SIMD batch opcodes (`VectorDot`, `VectorAdd`, `VectorMul`), and hardware-accelerated matrix transforms.

*(For detailed sprint-by-sprint release notes, see [changelog.md](changelog.md).)*

---

## ⚡ 60-Second Quickstart

Get KnotenCore running in less than a minute:

```bash
# 1. Clone the repository
git clone https://github.com/holgerbaer-bl/KnotenCore.git
cd KnotenCore

# 2. Build the headless workspace
cargo build --release

# 3. Run the automated test suite (342/342 passing)
cargo test --workspace --no-default-features

# 4. Start a headless JSON-RPC node on localhost
cargo run --release --bin run_knc -- --rpc-port 9000 --headless
```

---

## 🚀 CLI Usage

Execute a `.nod` or `.knoten` file using the native compiler and Register Stack-VM:
```bash
cargo run --bin run_knc -- <path_to.nod> [options]
```

To enable physical window creation, WGPU 3D rendering, and audio output:
```bash
cargo run --features ui --bin run_knc -- <path_to.nod> [options]
```

### Options:
* `--rpc-port <PORT>`: Starts KnotenCore in Headless Server Mode exposing the JSON-RPC 2.0 interface on `127.0.0.1:<PORT>` (`knc_compile`, `knc_execute`, `knc_yield_resume`, `knc_inspect_state`, `knc_agent_handshake`, `knc_agent_snapshot`, `knc_agent_restore`, `knc_mesh_discover`, `knc_mesh_peers`, `knc_agent_teleport`, `knc_task_submit`, `knc_task_status`, `knc_task_cancel`, `knc_task_steal`, `knc_mesh_metrics`, `knc_store_put`, `knc_store_get`, `knc_store_sync`, `knc_swarm_elect`, `knc_swarm_roles`, `knc_swarm_quorum`, `knc_swarm_request_vote`, `knc_swarm_heartbeat`, `knc_isolate_reload`).
* `--ws-port <PORT>`: Starts KnotenCore in Headless Server Mode exposing an RFC 6455 persistent WebSocket RPC transport on `127.0.0.1:<PORT>` with real-time `VmEvent` streaming (`knc_event`).
* `--headless`: Explicitly enforces headless execution mode. In pure headless builds (default `default = []`), physical window creation and WGPU graphics context initialization are bypassed automatically. UI AST nodes execute safely via no-op stubs.
* `--allow-read`: Enables sandboxed File I/O read permissions.
* `--allow-write`: Enables sandboxed File I/O write permissions.
* `--allow-net`: Enables sandboxed network fetch operations.
* `--output-format json`: Routes all compilation, type-checking, and runtime execution fault outputs to a structured, machine-readable JSON format on `stdout`:
  ```json
  {"status": "error", "errors": [{"code": "ERR_RUNTIME_FAULT", "message": "FFI Fault: <msg>", "agent_hint": "..."}]}
  ```

---

## 🤖 AI-Readiness & Architectural Immunity

KnotenCore is purpose-built for autonomous AI agents. Every node and native function is formally specified, machine-validated, and anchored against structural drift:

| Artifact | Path | Purpose |
|----------|------|---------|
| **EBNF Grammar** | [`docs/LANGUAGE_REFERENCE/nod_grammar.ebnf`](docs/LANGUAGE_REFERENCE/nod_grammar.ebnf) | Normative structural grammar of every `.nod` JSON node. Eliminates ambiguity for LLM code generation. |
| **JSON Schema** | [`docs/LANGUAGE_REFERENCE/node_types.json`](docs/LANGUAGE_REFERENCE/node_types.json) | Full Draft-07 JSON Schema with `additionalProperties: false` on every object node. **Hallucinated fields are rejected at runtime.** |
| **Function Registry** | [`docs/LANGUAGE_REFERENCE/native_functions.json`](docs/LANGUAGE_REFERENCE/native_functions.json) | Machine-readable registry of every native FFI function (30+), with parameter types, return types, required permissions, and live AST call examples. |
| **Anti-Pattern Guide** | [`docs/LANGUAGE_REFERENCE/examples/99_antipatterns.nod`](docs/LANGUAGE_REFERENCE/examples/99_antipatterns.nod) | 10 explicit DO/DON'T patterns for AI agents covering wrong node names, bare scalars, hallucinated functions, and ExternCall misuse. |
| **Error Catalog** | [`docs/LANGUAGE_REFERENCE/error_catalog.json`](docs/LANGUAGE_REFERENCE/error_catalog.json) | Registry of execution fault codes and self-healing hints for AI agents. |
| **Semantic Anchoring** | `#ANCHOR:` | Standardized machine-readable source anchors (`CORE_TYPES_SOF`, `GPGPU_ASYNC_CHANNEL`) coordinate AI refactorings synchronously via the `llm.md` routing hub. |
| **AI Agent Guide** | [`llm.md`](llm.md) | Routing document directing agents to the authoritative references above and documenting all engine constraints. |

---

## 🎯 AI-Readiness Benchmark — 20/20 on internal test set (v2.24.5)

> Tested with a single agent (Antigravity/Claude) against 20 custom scenarios. Not an independent benchmark; no comparison values from other DSLs. PRs with additional/stricter test cases are welcome.

---

## ⏱️ Performance & Optimization Benchmarks

KnotenCore delivers predictable, bare-metal execution performance through its AOT Bytecode Stack-VM and SIMD vector pipeline:

### 1. Leibniz Pi Estimation (1,000,000 Iterations)
Evaluating heavy floating-point arithmetic (`Mul`, `Add`, `Div`, `While`, `Assign`) demonstrates consistent AOT speedup:
- **Tree-Walker Evaluator:** ~1,914 ms
- **AOT Stack-VM:** ~1,580 ms (**1.21x** native speedup out-of-the-box)

### 2. SIMD Vector Dot Product (100,000 Elements)
Contiguous numeric buffer memory representations (`Vec<f64>`) with batch opcodes (`VectorDot`, `VectorAdd`, `VectorMul`) leverage hardware auto-vectorization, processing vector lanes in a single CPU pass.

Run the reproducible benchmark suite locally:
```bash
cargo run --bin knoten_bench
```
*(For detailed benchmark methodologies and hardware profiles, see [docs/BENCHMARKS.md](docs/BENCHMARKS.md).)*

---

## Runtime Architecture (Three-Crate Workspace)

KnotenCore operates as a strictly separated, circular-dependency-free multi-crate ecosystem:

```
JSON-AST (.nod)  ->  Parser  ->  AST (Node enum inside knoten_core_types)
                                    |
              +---------------------+---------------------+
              |                                           |
        JIT Executor                              AOT VM Compiler
   (Inside aether_compiler)                   (Inside aether_compiler)
              |                                           |
    egui / WGPU Render Layer                      Flat Opcodes
   (Physical Repr. Layer)                     Stack-VM Core (ALU)
```

| Crate / Module | Role |
|---|---|
| **`knoten_core`** | **Facade** — Thin top-level crate; functions as a re-export facade for seamless workspace integration. |
| **`aether_compiler`** | **Engine Core** — Houses the autonomous JIT graph executor, the AOT bytecode compiler, and the Stack-VM for allocation-free ALU instruction processing. |
| **`knoten_core_types`** | **Sole Source of Truth** — Houses exclusively the pure data certificates (`Node`, `OpCode`, `SimdOp`) free of cross-crate logic coupling. |
| `src/audio.rs` | **Audio Engine (Live / Polyphonic Multi-Waveform Synth)** — Multi-channel synthesis via `rodio` with Sine, Sawtooth, Square, and Triangle waveform shaping plus ADSR envelope modulation; isolated sinks per channel, `PlayNote`/`StopNote` in the AOT path. |
| `src/vm/machine.rs` | **GPGPU Streaming** — Continuous Shader Vector Streaming with dynamic workgroup alignment; structured Array flattening for particle position/velocity recycling between iterations. |
| `src/bin/knoten_lsp.rs` | **Language Server (LSP)** — `tower-lsp` server for real-time linter validation, hover diagnostics, and structured particle stride enforcement directly in the editor. |
| `src/vm/machine.rs` | **Multi-Threaded Isolate Scaling (v1.5.0)** — `VMIsolate` spawns VM instances on independent OS threads. Lock-free RPC via `MAILBOX_REGISTRY`. Work-stealing scheduler with `WORK_STEALING_QUEUES`. Zero-Allocation FFI Bridge. Isolate-Bound Local Heap. DashMap Resource Grid. Live Hot-Swap Code Reloading. Agent Telemetry Channel: routing real-time runtime faults into structured JSON feedback registers for autonomous LLM self-correction with pre-fault snapshot rollback mechanics. Lockless Shared-Memory Virtual Buses: zero-copy Inter-Isolate DMA via Arc references and concurrent sharded maps, enabling isolates to expose large dynamic structures without serialization or payload replication. |

---

## Audio Engine (Sprints 220–227)

KnotenCore features a fully activated, polyphonic multi-waveform synthesizer with ADSR envelope shaping:

- **Async AudioThread**: Dedicated background thread with `rodio` output stream, decoupled via `mpsc::channel`. Playback commands are fire-and-forget — zero frame budget impact.
- **Multi-Waveform Synthesis**: `Waveform { Sine, Sawtooth, Square, Triangle }` — raw `f32` sample generation per oscillator shape via `generate_sample()`. Compiles from `.knoten` DSL via `PlayNote(channel, freq, duration, waveform)` → `OpPlayNote` in the AOT path.
- **ADSR Envelope Modulation**: Linear-phase Attack-Decay-Sustain-Release shaping via `adsr_amplitude()`. All phases use `.max(1)` guards to prevent division by zero. Compiler injects sensible defaults (5ms/20ms/0.7/100ms) — no DSL breakage.
- **Polyphonic Channels**: Per-channel `synth_sinks: HashMap<usize, Sink>` with stop/replace semantics via `StopNote(channel)`.
- **Edge-Case Guarantees**: 0 Hz frequency, negative ADSR times, and zero-duration envelopes produce no panics — bounded to `[0.0, 1.0]` amplitude range.

## GPGPU Compute & Native Math (Sprints 229–234)

- **Continuous Shader Vector Streaming**: `DispatchComputeLoop` dispatches compute shaders with dynamic workgroup alignment (`x = max(1, n).div_ceil(64)`). Results are recycled with zero-allocation swap for flat data, structured `RelType::Array` flattening for particle position/velocity vectors.
- **Lock-Free Readback**: Per-shader `crossbeam_channel::bounded(1)` channels. Render thread uses non-blocking `try_send()`, VM thread uses `try_recv()` with spin-poll after mutex guard drop.
- **SIMD Matrix Transpose**: `math_matrix_transpose(handle) -> handle` — native FFI with handle-based `MATRIX_REGISTRY` storage, hardware-accelerated via `glam::Mat4::transpose()`.
- **LSP Particle Diagnostics**: Real-time validation of `DispatchComputeLoop` inputs — enforces stride alignment (multiples of 6 or 7) with `ERR_PARTICLE_STRIDE` error markers mapped to exact editor positions via `find_range()`.
- **SIMD Matrix Injection (Sprint 236)**: `OpDispatchComputeLoop` now accepts an optional matrix handle. If a valid `glam::Mat4` is bound, the compute loop applies `transform_point3`/`transform_vector3` to particle position/velocity strides in-place before GPU dispatch — zero additional allocations.

---

## Performance & Data Processing Architecture

KnotenCore scales natively via high-performance AOT compilation loops, zero-copy collection mutations, and deterministic SIMD vector lanes. 

### Idiomatic Live Telemetry Compilation Example
```javascript
// Sprint 224: Continuous frame-synchronous GPGPU streaming loop implementation
// Sourced via JSON-AST from https://knotencore.de/

let payload = json_parse(file_read("examples/telemetry_cache.json"));

// Zero-allocation object property traversal via compiler inlining
let cpu_usage = payload.system.metrics.cpu;
let ram_usage = payload.system.metrics.ram;

// Inject structures into the egui rendering thread via data-bound UI components
ui_init_window(800, 400, "KnotenCore Live Telemetry Monitor");

while (true) {
    UIWindow("Dashboard", "System Status") {
        ui_bar_chart("CPU History (Last 10 Runs)", cpu_usage);
        ui_progress_gauge("RAM Saturation", ram_usage, 0.0, 100.0);
    };
    
    ui_present();
    sleep(16); // Throttling execution cycle to match a steady ~60 FPS VSync target
}
```

---

## Compliance & Community Flow

This repository maintains absolute structural and version integrity. Every sprint is planned, rigorously executed, evaluated across native unit/integration tests, explicitly documented within `changelog.md`, and natively pushed to this repository by autonomous agents.

### Community Guidelines

Open-source contributors and autonomous agents interacting with this framework must strictly abide by our repository documentation:

- Review [CONTRIBUTING.md](CONTRIBUTING.md) to understand the AOT Stack Machine and Sandbox constraints before submitting `PULL_REQUEST` templates.
- Consult [SECURITY.md](SECURITY.md) to privately report FileSystem/FFI escapes.
- Follow the official [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

Reference `llm.md` for strict machine-readable constraints regarding runtime architecture and OS bindings.

---

**[https://knotencore.de/](https://knotencore.de/) — The Official Engineering & Telemetry Control Hub**
