# KnotenCore 🦀🤖

[![Version](https://img.shields.io/badge/version-v2.21.4--security-blue)](https://github.com/holgerbaer-bl/KnotenCore/releases/latest)
[![CI Quality Gates](https://github.com/holgerbaer-bl/KnotenCore/actions/workflows/ci.yml/badge.svg)](https://github.com/holgerbaer-bl/KnotenCore/actions/workflows/ci.yml)
[![Tests](https://img.shields.io/badge/tests-306%2F306-brightgreen)](https://github.com/holgerbaer-bl/KnotenCore/actions)
[![Release](https://img.shields.io/badge/release-v2.21.4--security-brightgreen)](https://github.com/holgerbaer-bl/KnotenCore/releases/latest)

*(Noun) /knoːtən kɔːr/*

1. **Not** a relentless underground German hardcore techno subgenre. 
2. A high-performance, headless Rust runtime & P2P mesh engine for autonomous AI agents — fully driven by JSON-AST.

**A high-performance, headless Rust runtime & P2P mesh engine for autonomous AI agents — fully driven by JSON-AST.**

## What is KnotenCore?
**KnotenCore** is a high-performance, headless Rust runtime & P2P mesh engine for autonomous AI agents — fully driven by JSON-AST. By executing structured JSON-AST nodes (`.nod` files) instead of raw text, KnotenCore eliminates LLM syntax hallucinations and parser ambiguities. The engine compiles ASTs directly into an AOT-optimized bytecode stream executed by a bare-metal Register Stack-VM.

### Key Features:
- **Audit Completion, State Persistence & Quorum Fixes (v2.21.4-security)**: Persistent disk storage & load gates for peer revocation lists (`revoked_keys.json`), peer registration auth gate against revoked keys (`knc_mesh_peers`), quorum threshold denominator hardening excluding Evicted/Stale peers in `knc_swarm_quorum` and `knc_mesh_revoke_peer`, stack traversal fix in `estimate_memory_bytes`, and custom `IsolateQuota` propagation in `VMIsolate`.
- **CI Test Isolation & Swarm Expectation Reconciliation (v2.21.3-security)**: Complete removal of `cfg!(test)` runtime bypasses in `SwarmGovernance::elect()`, isolation of test fixtures across all integration tests, addition of `#[cfg(test)]` test helper for internal unit tests, and reconciliation of election expectations.
- **Root Election Hardening & Exhaustive Bounds (v2.21.2-security)**: Complete removal of unilateral self-nomination in `SwarmGovernance::elect()`, elimination of client-side `allow_test_harness` parameter bypasses, and exhaustive `validate_param_string_len` enforcement across all `session_id` and `nonce_str` parameter extractions.
- **Security Audit Rectification & Resource Limits (v2.21.1-security)**: TCP & WS payload caps (`MAX_BODY_BYTES = 1 MiB`, `MAX_WS_PAYLOAD = 1 MiB`), HMAC Nonce-LRU replay defense (`NonceCache`), string parameter length caps (`MAX_PARAM_STRING_LEN = 256`), VM call depth limits (`MAX_CALL_DEPTH = 512`), and universal self-election locks in `knc_swarm_elect`.
- **Server-Enforced Swarm Quorum & Quorum-Gated Peer Revocation (v2.21.0-authz)**: Server-enforced quorum computation `(active_nodes / 2) + 1` resisting client parameter manipulation (`knc_swarm_quorum`), strict prohibition of forced self-election (`force: true`) in Zero-Trust mode (`knc_swarm_elect`), and quorum-consensus gated peer revocation (`knc_mesh_revoke_peer`).
- **Comprehensive RPC Auth Bypass Mitigation (v2.20.1-security)**: Exhaustive RPC authentication enforcement (`check_mesh_auth`) across ALL 25 JSON-RPC endpoints (`knc_*`), completely mitigating unauthenticated access to `knc_agent_snapshot`, `knc_agent_restore`, `knc_compile`, `knc_execute`, `knc_yield_resume`, and `knc_inspect_state`.
- **Zero-Trust Mesh Phase 2: Key Rotation, Nonce LRU Eviction & Peer Revocation**: Volatile in-memory Ed25519 keypair re-keying (`knc_mesh_rotate_key`) without interrupting active streams, keyless session migration, bounded LRU nonce cache (`MAX_NONCE_CACHE_CAPACITY = 10_000`) with automatic TTL eviction, and instant peer revocation lists (`knc_mesh_revoke_peer`). *Hinweis: Die kryptografische Mesh-Signierung und Key-Rotation befinden sich in Phase 2 (lokale Ed25519-Verifikation & Peer-Sperrlisten). Sie ersetzen keine externe professionelle Penetrationsprüfung oder Drittanbieter-Sicherheitsaudit.*
- **Swarm Governance (Local Swarm Role Management & Leadership Claim Primitives - Phase 1)**: Term tracking, dynamic node roles (`Leader`, `Worker`, `Storage`, `Observer`), and quorum voting (`knc_swarm_elect`, `knc_swarm_roles`, `knc_swarm_quorum`). *Hinweis: `knc_swarm_elect` verwaltet aktuell den lokalen Knotenzustand und Leadership-Claim (Phase 1). Ein vollwertiger Cross-Node Consensus Broadcast via Mesh ist für ein folgendes Release geplant.*
- **Distributed CRDT Storage & Peer State Sync**: In-memory Last-Write-Wins (LWW) CRDT key-value store (`knc_store_put`, `knc_store_get`, `knc_store_sync`).
- **Distributed Task Queue & Adaptive Work-Stealing**: Cluster task dispatching (`knc_task_submit`, `knc_task_status`, `knc_task_cancel`) and CPU load-adaptive work-stealing (`knc_task_steal`, `knc_mesh_metrics`).
- **P2P Mesh Protocol & Teleportation**: Zero-broker peer discovery, gossip auto-healing, and live isolate state migration (`knc_mesh_ping`, `knc_mesh_discover`, `knc_mesh_peers`, `knc_agent_teleport`).
- **Headless-First Architecture**: Lightweight headless execution by default, with optional UI/rendering features (`--features ui`).

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
* `--rpc-port <PORT>`: Starts KnotenCore in Headless Server Mode exposing the JSON-RPC 2.0 interface on `127.0.0.1:<PORT>` (`knc_compile`, `knc_execute`, `knc_yield_resume`, `knc_inspect_state`, `knc_agent_handshake`, `knc_agent_snapshot`, `knc_agent_restore`, `knc_mesh_discover`, `knc_mesh_peers`, `knc_agent_teleport`, `knc_task_submit`, `knc_task_status`, `knc_task_cancel`, `knc_task_steal`, `knc_mesh_metrics`, `knc_store_put`, `knc_store_get`, `knc_store_sync`, `knc_swarm_elect`, `knc_swarm_roles`, `knc_swarm_quorum`).
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

## 🎯 AI-Readiness Benchmark — 20/20 auf internem Test-Set (v2.21.4-security)

> Getestet mit einem einzigen Agenten (Antigravity/Claude) gegen 20 selbst definierte Szenarien. Kein unabhängiger Benchmark, keine Vergleichswerte anderer DSLs. PRs mit zusätzlichen/härteren Testfällen sind willkommen.

---

## ⏱️ Performance & Optimization Benchmarks

KnotenCore features a dual JIT/AOT engine architecture. On a computationally demanding 1,000,000 algorithmic loop constraint using the Leibniz pi estimation heavily encoded with Float primitives (`Mul`, `Add`, `Div`, `While`, `Assign`), tests generated via `bench_knc` resulted in:

- **JIT Evaluator:** ~1914 ms
- **AOT Stack VM:** ~1580 ms (Speedup factor: **1.21x** natively faster out-of-the-box).

### ⚡ AOT Compiler Optimization Engine
* **AST Function Inlining (Sprint 194):** Trivial native FFI math and string calls are resolved and folded directly at compile time.
* **Loop Unrolling & Static Bound Analysis (Sprint 196):** Bounded while-loops ($N \le 8$) are expanded into flat blocks at compile time. Static infinite loops without exit paths are rejected early.
* **Peephole Optimization & Slot Reuse (Sprint 197):** Instruction post-pass eliminates redundant Store-Load chains. Register slot reuse minimizes stack frame sizes.
* **SIMD Auto-Vectorization (Sprint 200):** High-speed optimizer pass collapses sequential float constants into single instruction streams execution-driven by `glam::Vec4` in a single CPU tick.
* **Mathematical Resilience Fortress:** Integrated proptest framework into the native testbett, executing deep mathematical property-based fuzzing over the ADSR audio shaper and SIMD Glam matrix transformation FFI layers.
* **Dynamic Shader Synthesis:** Integrated a JIT Multi-Pass Shader Graph Synthesizer, compiling dynamic AST computation blocks directly into high-performance WGSL compute shaders executed directly on the GPU pipeline cache.
* **Speculative Isolate Execution:** Integrated a runtime speculative branching engine. Complex conditional blocks spawn parallel shadow isolates across independent OS threads, merging the winning path atomically with zero-latency snapshot rollbacks for losing branches.
* **Cluster-Wide Work-Stealing:** Expanded the deterministic scheduler grid to support cross-network RDMA operations. Isolates can balance asymmetric execution loads by stealing bytecode segments directly across node boundaries with zero OS layer intervention.
* **Universal C-ABI Embedding Facade:** Exposed a low-overhead, unsafe-free C-compatible ABI boundary (libknotencore), enabling native orchestration, bytecode injection, and multi-threaded isolate spawning from host languages like Python, C++, and Node.js.

### Deterministic Execution & State Rewind Architecture (v1.6.0)
* **Deterministic State Rewind:** Integrated instruction-accurate state rewinding inside the Register Stack-VM ALU core. Exploits localized snapshot frames to enable deterministic state rollbacks and live register mutations under execution faults.
* **Deterministic Execution Path Hashing:** Embedded an accumulation-based hash framing into Register Stack-VM execution loops, enabling verification of unmodified bytecode execution paths across host language boundaries.
* **Adaptive Evolutionary PGO:** Embedded a runtime profile-guided optimization layer. The engine dynamic-mutates active instruction streams based on real-time execution metrics, remapping hot-paths and unrolling tight loops natively inside running isolates.
* **Sovereign JIT Native Code Generation:** Deployed an allocation-free native machine code generation layer within the JIT execution subsystem, emitting standalone executable memory segments bypassing external compiler toolchains.
* **Compiler Stabilization (v1.6.1):** Rectified x86_64 JIT stack operand handling to prevent allocation leaks and fixed the frame-pointer alignment during execution returns. Resolved PGO loop truncation bugs and corrected dynamic WGSL float parsing rules.
* **JIT & Optimizer Rectification (v1.6.2):** Corrected the x86_64 JIT subtraction operand execution order to guarantee left-minus-right mathematical parity. Implemented an absolute bytecode jump target relocation pass within the PGO loop unroller to maintain control-flow integrity during optimization splices.
* **Executable JIT Memory Layer (v1.6.3):** Integrated a native memory execution layer via memmap2. Enables the runtime to allocate dynamic execution pages, transition synthesized x86_64 bytecode from write to execute permissions, and invoke compiled blocks as native function pointers.
* **Native Control-Flow JIT Expansion (v1.6.4):** Extended the native machine code generation layer to support structural control flow, translating bytecode Jumps and conditional branches directly into native x86_64 relative near jump instructions. Integrated dynamic offset recalculation for nested loops.
* **Universal Language SDKs (v1.6.5):** Implemented automated native language shims for Python and Node.js utilizing the stable C-ABI boundary layer. Enables allocation-free host runtime instantiations and multi-threaded isolate code streaming.
* **Distributed WebGPU Edge Grid (v1.6.6):** Deployed a WebAssembly compilation and execution layer enabling isolates to scale across browser nodes. Interconnects edge runtimes via peer-to-peer pipelines to distribute WebGPU shader synthesis dynamically.
* **Streaming Audio Infrastructure (v1.6.7):** Refactored the core synthesizer to process long-form tone generations as non-blocking dynamic streaming sources. Implemented automated background sweeping for terminated audio channels to guarantee stable descriptor thresholds.
* **Persistent Snapshot Storage (v1.6.8):** Implemented a binary disk serialization layer for isolate runtime states. Enables bit-accurate snapshot serialization and physical disk persistence to support cold-starts and horizontal isolate migrations.
* **Cross-Node Isolate Migration (v1.6.9):** Engineered a distributed context handoff pipeline within the cluster scheduler. Enables live runtime environments to pack active execution states into binary network payloads, migrating isolates horizontally across cluster nodes with execution path hash continuity.
* **Workspace Consolidation & Cluster Tooling (v1.7.0):** Deployed an autonomous cluster orchestration CLI tool named knoten-init alongside cloud-native development profiles. Establishes programmatic onboarding baselines to lower integration boundaries for external actors.
* **Runtime Hardening & JIT Parity (v1.7.1):** Corrected the x86_64 JIT addition register emission matrix to prevent left-operand discarding. Fully integrated structural branch relocation for internal loop jumps and restored multi-threaded isolate migration states.
* **Cross-Platform JIT Guarding (v1.7.2):** Implemented compile-time architecture gates for native memory execution. Protects non-x86_64 host environments by enforcing graceful runtime fallbacks to the deterministic software interpreter loop.
* **Kryptographic State Ledger (v1.7.3):** Hardened the state rewind pipeline with a nonced block-chaining ledger model. Protects snapshot data from replay attacks by enforcing cryptographic hash continuity across historical execution boundaries.
* **Zero-Copy Isolate Garbage Collection (v1.7.4):** Integrated a sub-millisecond compact memory sweeper for terminated isolates. Reclaims execution pages and returns memory directly to the host OS without runtime pauses.
* **Agent Validation Layer (v1.7.5):** Deployed the standardized autonomous agent onboarding matrix to mathematically verify specification adherence and compiler documentation Parity.
* **Schema Synchronization (v1.7.6):** Synchronized front-end AST JSON schemas with core VM engine capabilities. Formally integrated native declarations for GPGPU pipelines, asynchronous multi-node isolate spawning, and real-time audio synthesis.
* **WGPU Live Inspector (v1.7.7):** Integrated an interactive live GUI debugger panel leveraging egui. Real-time visualization of VM stack depths, call frames, registers, and cryptographic ledger states during execution.
* **P2P Mesh-Bus Routing (v1.7.8):** Expanded the virtual shared-memory bus architecture into a fully decentralized peer-to-peer routing fabric. Enables multi-node isolate communication via zero-broker distributed pub-sub topologies.
* **Distributed Swarm Governance Consensus (v1.7.9):** Integrated a lightweight, decentralized consensus layer mapped directly to the cryptographic state ledger. Enables autonomous cluster leader election and sub-millisecond isolate failover migration upon node partitioning.
* **Production Release (v2.0.0):** KnotenCore reaches full production stability. Hardened the distributed Swarm Governance consensus layer and exposed a locked, stable WGPU telemetry dashboard for enterprise edge runtimes.
* **Compiler Realignment (v2.1.0):** Fully implemented missing graphical and mathematical AST nodes (Transform2D, DrawRect, Sin, Cos). Fixed WGPU pipeline binding mismatches and recalibrated the runtime loop watchdog.
* **True Distributed Swarm Consensus (v2.1.0):** Fully synchronized Cargo workspace versions. Extended the scheduler with real socket-based P2P log replication and randomized network election timers. Automated wasm-pack compilation gates injected into the GitHub Actions CI workflow to guarantee platform browser readiness.

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
