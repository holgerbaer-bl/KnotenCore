# KnotenCore — AI Agent Reference (Routing Document) - v2.3.0-headless-alpha Release

> **System Instruction for LLM Code Agents**
>
> You are a **System Architect**, not a game developer. KnotenCore is a **Headless-First Deterministic AI-Native Execution Runtime** — you feed it structured JSON logic (`Node` objects), and its AOT compiler turns that into flat bytecode executed by a bare-metal Register Stack-VM. Your output is machine instructions, not imperative scripts.
>
> This document is a **routing hub**. Its sole purpose is to direct you to the authoritative
> machine-readable sources in `docs/LANGUAGE_REFERENCE/`. Do **not** invent node names or
> argument shapes. Always validate against the schema before generating code.

---

## 🎯 AI-Readiness Benchmark — Sprint 305

KnotenCore has a **public, reproducible AI-Readiness Benchmark**. If you are an external LLM agent
generating `.nod` programs, your output can be tested against 20 standardised tasks.

**Before you generate any code, read: [`benchmark/README.md`](benchmark/README.md)**

**AG Baseline Score: 20/20 (100%)** — see [`benchmark/results/ag_baseline.md`](benchmark/results/ag_baseline.md)  
**Current Engine Version: v2.3.0-headless-alpha** — Headless-first architecture, optional `ui` feature gate (`wgpu`, `winit`, `egui`), no-op UI stubs, instruction limit guard (1,000,000 opcodes -> `ERR_SANDBOX_TIMEOUT`), and 16MB memory threshold (`ERR_MEMORY_LIMIT_EXCEEDED`).  
*Note: All AST Nodes map gracefully in the VM Compiler. In headless mode (or when built without the `ui` feature), UI nodes execute safely via no-op stubs without breaking compilation.*

---

## ⚡ Primary References (Read These First)

| Document | Purpose |
|----------|---------|
| [`docs/LANGUAGE_REFERENCE/node_types.json`](docs/LANGUAGE_REFERENCE/node_types.json) | **Normative JSON Schema** — every AST node, field name, and type constraint. `additionalProperties: false` on all objects. Hallucination-proof. |
| [`docs/LANGUAGE_REFERENCE/nod_grammar.ebnf`](docs/LANGUAGE_REFERENCE/nod_grammar.ebnf) | **Formal EBNF Grammar** — structural grammar of the `.nod` JSON AST. |
| [`docs/LANGUAGE_REFERENCE/native_functions.json`](docs/LANGUAGE_REFERENCE/native_functions.json) | **Native Function Registry** — every FFI function by module, with parameter types, return types, required permissions, and live AST examples. AI agents MUST only call functions listed here. |
| [`docs/LANGUAGE_REFERENCE/examples/99_antipatterns.nod`](docs/LANGUAGE_REFERENCE/examples/99_antipatterns.nod) | **Anti-Pattern Reference** — 10 explicit DO/DON'T patterns for AI code generation. Read before emitting any `.nod` code. |
| [`docs/LANGUAGE_REFERENCE/error_catalog.json`](docs/LANGUAGE_REFERENCE/error_catalog.json) | **Error Catalog** — registry of execution fault codes and self-healing hints for AI agents. |
| [`docs/KNOTEN_SPEC.md`](docs/KNOTEN_SPEC.md) | **KNOTEN_SPEC.md** — Human-readable language reference, derived from `knoten_core_types/src/ast.rs`. If spec and source diverge, `knoten_core_types/src/ast.rs` wins. |
| [`knoten_core_types/src/ast.rs`](knoten_core_types/src/ast.rs) | **Rust source of truth** — `pub enum Node` is the canonical definition in the shared types crate. **Anchor: `#ANCHOR:CORE_TYPES_SOF`**. If schema and source diverge, source wins. |

---

## Architecture in 60 Seconds

KnotenCore is a **Deterministic AI-Native Execution Runtime** written in Rust. You author a `Node` tree (as JSON). The engine selects the optimal path:

```
JSON-AST (.nod)  →  Parser  →  AST (Node enum)
                                  │
                    ┌─────────────┴──────────────────────┐
                    │                                    │
             JIT Executor                       AOT VM Compiler
   (side-effects, I/O, UI rendering)      (pure math / computation loops)
                    │                                    │
        egui / WGPU / Audio FFI               Flat Opcodes → Stack-VM
     (Physical Representation Layer)         (GC-free bare-metal ALU)
```

- **Pure computation** (`Add`, `Sub`, `Mul`, `Div`, `While`, comparison ops) → **AOT path** — compiles to a flat opcode stream, executed by the Stack-VM with no allocations in the hot path.
- **Side-effects & UI** (`ExternCall`, window ops, WGPU render, audio) → **JIT path** — evaluated by the Executor with full permission sandboxing.
- **GPGPU Compute** (`LoadComputeShader`, `DispatchCompute`) → **Native WGPU integration** for massive parallel data processing.
- **GPGPU Compute Readback** (Sprint 213) → Non-blocking crossbeam channel between render thread (producer) and VM thread (consumer). `try_recv()` returns immediately — no frame budget impact. **Anchor: `#ANCHOR:GPGPU_ASYNC_CHANNEL`** in `aether_compiler/src/natives/registry.rs`.
- **Retained-Mode Scene Graph** (Sprint 159) → Scripts spawn entities (`registry_spawn_cube`) and update transforms asynchronously; WGPU renders the persistent `SceneGraph` autonomously.
- **Retained-Mode 2D UI** (Sprint 162) → Scripts set a `UIButton`/`UITextInput`/`UILabel`/`UIVBox`/`UIHBox` tree once via `UpdateUI`; egui renders it at 60 FPS. Events route back via `registry_ui_poll_button(label)` (Bool) and `registry_ui_read_text(key)` (String).
- **UI→3D Value Bridging** (Sprint 163) → `registry_parse_float(String) -> Float` safely converts `UITextInput` strings to floats for use as 3D coordinates. Returns `0.0` on invalid input.
- **Retained-Mode Physics & Raycasting** (Sprint 164) → 3D entities have automatically synchronized AABBs. Use `registry_get_clicked_entity(win) -> Int` for 3D mouse picking. Use `registry_check_collision(id1, id2) -> Bool` for intersection checks.
- **Retained-Mode Textures** (Sprint 165) → External image files are loaded via `registry_load_texture(path) -> Int`. The returned ID is passed to `registry_spawn_cube` or similar functions to automatically UV-map the object via a thread-safe `TEXTURE_CACHE`.
- **Math Standard Library** (Sprint 166/168) → Core trigonometric and mathematical functions (`math_sin`, `math_cos`, `math_tan`, `math_sqrt`, `math_abs`, `math_pi`) plus random number generation (`math_random(min, max)`) are provided natively via the FFI Bridge, enabling complex floating-point spatial computations and procedural generation directly within the VM loop.
- **Dynamic Lighting** (Sprint 167) → The WGSL shader implements Blinn-Phong shading with ambient light and up to 4 dynamic point lights. Use `registry_spawn_light(win, x, y, z, intensity) -> Int` to create a light and `registry_update_light_position(win, light_id, x, y, z)` to animate it. Light data is written per-frame to the camera UBO.
- **Resource Cleanup** (Sprint 169) → Entities can be destroyed at any time via `registry_destroy_entity(win, id)`. This removes the entity from the scene graph and physics world immediately. Textures and geometry remain cached for other entities. RAM and VRAM remain stable under continuous spawn/destroy cycles.
- **The Security Hotfix** (Sprint 179) → Hardened the runtime sandbox with FFI validation checks on texture loading, hidden debug panic hooks behind `debug_assertions`, routed VM unit tests through safe `Result` handlers, and forced `panic = "unwind"` in the release profile to preserve the FFI crash-protection shield.
- **Persistence & File I/O** (Sprint 183) → Native `file_read(path)` and `file_write(path, content)` under the `fs` bridge module. Both enforce `ffi_safety::validate_string` path guards and `allow_fs_read` / `allow_fs_write` permissions. Combine with `json_parse` / `json_stringify` for stateful dashboards that survive restarts.
- **Registry Refactoring** (Sprint 184) → Monolithic `registry.rs` split into `geometry.rs` (mesh generators), `physics.rs` (AABB collision/raycasting), and `scene.rs` (entities, lights, camera, spawn lifecycle). Dead Voxel code purged.
- **FFI Consolidation** (Sprint 185) → Removed `registry_read_file`, `registry_write_file`, `registry_get_ultimate_answer`. All sandboxed file I/O is now exclusively via `file_read` / `file_write` in the `fs` module. `SENT_MESHES` cache moved into `scene.rs` for self-contained mesh deduplication.
- **Core Purge** (Sprint 186) → Removed all 7 Voxel AST Node variants (`InitCamera`, `DrawVoxelGrid`, `LoadTextureAtlas`, `InitVoxelMap`, `SetVoxel`, `EnableInteraction`, `EnablePhysics`) from the compiler core, executor, validator, evaluator, and optimizer. Cleaned schemas, benchmarks, docs, and VSCode tooling.
- **WGPU Compute Pipeline** (Sprint 187) → Storage buffers wired up in `DispatchCompute`: inputs are serialized to `wgpu::Buffer` with bind group binding. Pipeline cache (`compute_pipelines`) confirmed operational. New `math_vector_scale(array, factor)` and `math_matrix_transform(matrix, vector)` FFI functions added.
- **Sandboxed Time Module** (Sprint 188) → `time_get_string()` returns formatted wall-clock time (`YYYY-MM-DD HH:MM:SS`). `time_utc_timestamp()` returns UTC epoch seconds. Uses `chrono = "0.4"` for timezone-safe date/time handling.
- **Databound UI Components** (Sprint 189) → `ui_bar_chart(label, data)` renders native bar charts from numeric arrays. `ui_progress_gauge(label, value, min, max)` renders animated progress bars. Thread-safe buffer queues updated each frame.
- **Iron Shield Hardening** (Sprint 190) → Symlink blocking via `symlink_metadata` in both path validators. Network domain whitelist + 5-second timeout on all HTTP calls. GPU buffer keys sorted for deterministic memory layout. `catch_unwind` guards on all FFI/bridge dispatch paths.
- **Multi-Waveform Audio Synth** (Sprint 226) → `PlayNote(channel, freq, duration, waveform)` with `Waveform { Sine, Sawtooth, Square, Triangle }` compiles to `OpPlayNote` in the AOT path. The audio thread generates raw `f32` samples per waveform shape from the VM stack. LSP enforces arity (`ERR_AUDIO_ARITY`, `ERR_STOP_AUDIO_ARITY`) and waveform bounds (`ERR_AUDIO_WAVEFORM_BOUNDS`).
- **ADSR Envelope Modulation** (Sprint 227) → `adsr_amplitude(t_ms, attack, decay, sustain, release, total_ms)` computes a linear-phase amplitude multiplier (0.0–1.0) per sample. Four phases: Attack (0→1 ramp), Decay (1→sustain ramp), Sustain (constant hold), Release (sustain→0 ramp). All segments use `.max(1)` guards to prevent division by zero. Boundary guarantees: 0 Hz, negative times, and zero-duration parameters produce no panics — envelope bounded to `[0.0, 1.0]`. Compiler injects defaults (5ms/20ms/0.7/100ms) into the AOT stack.
- **SIMD Matrix Algebra** (Sprint 233) → `math_matrix_transpose(handle: Int) -> Int` in the `math` FFI bridge module. `MATRIX_REGISTRY` (`OnceLock<Mutex<HashMap<i64, Mat4>>>`) with `AtomicI64` handle counter. `glam::Mat4::transpose()` provides hardware-accelerated SIMD transposition. Fault on missing handle.
- **GPGPU Recycling Hotpath** (Sprint 234) → `OpDispatchComputeLoop` and JIT evaluator use zero-allocation result recycling: flat `Vec<RelType>` results are swapped directly (`inputs = result`), nested `RelType::Array` elements are flattened via `clear()+extend()`. Dynamic workgroup alignment: `x = max(1, n).div_ceil(64)`. LSP enforces particle stride (`ERR_PARTICLE_STRIDE`) on `DispatchComputeLoop` inputs — validates alignment to multiples of 6 or 7.
- **SIMD Matrix Injection** (Sprint 236) → `OpDispatchComputeLoop` accepts an optional matrix handle on the VM stack. If a valid `glam::Mat4` is resolved from `MATRIX_REGISTRY`, the loop applies `transform_point3()` to particle positions and `transform_vector3()` to velocity vectors in-place before each GPU dispatch. Stride detection (6 or 7) is automatic. Zero-allocation: mutation happens on the existing `inputs` slice via `apply_matrix_to_inputs()`. Default handle is -1 (no transformation). Compiler pushes a `Constant(-1)` to maintain stack layout.
- **All OS I/O** → sandboxed; permissions must be granted via CLI flags (`--allow-read`, `--allow-write`, `--allow-net`)
- **Language Server (LSP)** → `knoten_lsp` binary validates `.nod` JSON documents in real-time. The **VS Code Extension** automatically launches this server, flagging unknown opcodes (`ERR_UNKNOWN_NODE`) and JSON parse errors (`ERR_JSON_PARSE`) directly in the editor before they reach the runtime. Tracing output is visible in the VS Code *Output → knoten-lsp* channel.
- **GitHub Linguist** → `.nod` targets `JSON` and `.knoten` targets `JavaScript` for correct repository rendering.
- **GPU UI Hit-Testing** (Sprint 257) → `ui_hit_test.wgsl` compute shader offloads panel AABB intersection testing from CPU to GPU. `RenderCommand::UiHitTest` uploads panel bounds and mouse position to storage buffers, dispatches a single workgroup, and returns the hit panel index via compute readback.
- **Agent Latency Monitor** (Sprint 258) → `LATENCY_MONITOR: OnceLock<Mutex<HashMap<String, Vec<u128>>>>` tracks µs-precision command-to-response latencies. `registry_start_latency_timer(id)` / `registry_stop_latency_timer(id) -> Float` with `registry_get_avg_latency(id) -> Float` enabling LLM agents to measure their own "command-to-photon" response times.
- **Deterministic State Rollback** (Sprint 259) → `VMState { globals, stack, frames, ip, base_pointer }` captures a complete VM execution snapshot. `VM::snapshot() -> VMState` and `VM::rollback(state)` enable checkpoint-based fault recovery.
- **Multi-Threaded Isolate Scaling** (v1.5.0) → `VMIsolate { instructions, constants, isolate_id, mailbox }` wraps a complete VM execution context. `spawn_isolate()` creates a `std::thread::spawn` running `VM::run()` with fully isolated operands (stack, globals, frames) — zero cross-thread data races.
- **Lock-Free Mailbox RPC** (v1.5.0) → `registry_send_message(target_isolate_id: Int, message: Any) -> Bool` routes a `RelType` payload to the target isolate's crossbeam `Sender` via `try_send()`. `MAILBOX_REGISTRY: OnceLock<Mutex<HashMap<i64, Sender<RelType>>>>` maps isolate IDs to bounded(16) channels.
- **Deterministic Work-Stealing Scheduler** (v1.5.0) → `push_work_batch(isolate_id: Int, batch: Array)` donates `(OpCode, Vec<RelType>)` tasks to a global `WORK_STEALING_QUEUES` pool. `try_steal_work(thief_id: Int)` pops from victim queues. `VMIsolate::run()` auto-checks for stolen work when local instructions are empty.
- **Atomic Snapshot Synchronization** (Sprint 263) → `ISOLATE_SNAPSHOTS: OnceLock<Mutex<HashMap<i64, VMState>>>` provides non-blocking per-isolate checkpoint storage. `VMIsolate::run()` auto-stores a snapshot before execution and auto-rolls back on `Err` — Mutex held only for HashMap insert/lookup, never during VM execution.
- **Zero-Allocation FFI Bridge** (Sprint 265) → All `math`, `string`, and `wgpu` bridge handlers are stateless pure functions operating on borrowed `&[RelType]` references. No temporary `clone()` calls in the hot-path. Multiple `VMIsolate` threads can execute parallel FFI invocations without mutex contention — agents may freely issue high-frequency math calls from any isolate without synchronization overhead.
- **Isolate-Bound Local Heap** (Sprint 266) → `VMIsolate.local_heap: HashMap<String, RelType>` provides per-thread temporary storage for arrays, dictionaries, and dynamic structures. The heap is drained into `VM::globals` before execution and atomically freed when the thread's `JoinHandle` completes. Agents may allocate large data structures within isolates without impacting the global heap — ideal for particle simulations, graph traversals, and memory-intensive AI workloads.
- **Lock-Free Resource Handles** (Sprint 267) → `COUNTER_REGISTRY` utilizes atomic reference mapping. Isolates resolve native handles concurrently with zero-latency lock mitigation. `StatefulCounter` uses `AtomicI64` for lock-free `fetch_add`/`load` — eliminates mutex contention on `registry_increment` and `registry_get_value` hot-paths.
- **DashMap Resource Grid & Crate Decomposition** (Sprint 268) → Eliminates monolithic registry mutexes via DashMap integration. `COUNTER_REGISTRY` now uses `OnceLock<DashMap<usize, RegistryEntry>>` — 100% lock-free concurrent access across all registry operations. Crate machine.rs decomposed into isolated modules (`isolate.rs`, `scheduler.rs`, `snapshot.rs`) minimizing architectural merge conflicts.
- **Property-Based Testing & WASM CI-Fortress** (Sprint 269) → Integrates proptest into the runtime validation matrix to dynamically expose NaN/Inf boundary flaws within ADSR envelopes and SIMD matrix logic. Establishes native wasm-pack validation rules within the automated GitHub Actions pipeline.
- **Multi-Threaded Hot-Swap Code Reloading** (Sprint 270) → Activates runtime opcode vector manipulation directly targeting individual VMIsolate instances. Leveraging snapshot-recovery states, modified instruction streams are hot-swapped in real-time while adjacent threads sustain peak execution velocity.
- **Agent Telemetry Channel & Self-Healing** (Sprint 271) → Connects the execution watchdog directly to structured JSON feedback pipelines. Captures runtime faults natively and provides diagnostic error catalogs directly to autonomous code agents to trigger real-time code modifications.
- **Lockless Shared-Memory Virtual Buses** (Sprint 272) → Establishes zero-copy inter-isolate DMA routing. Threads share native data structures concurrently using atomic shared pointers, avoiding serialization bottlenecks under intensive data exchange.
- **Dynamic WGSL Shader Synthesis** (Sprint 273) → Empowers the runtime to natively compile JSON-AST mathematical expression graphs into optimized WebGPU compute shaders at runtime, bypassing CPU-bound evaluation paths.
- **Speculative Branch Execution** (Sprint 274) → Evaluates dual control flow paths concurrently using transient speculative shadow isolates. Automatically commits valid execution tracks and prunes alternative paths via low-overhead snapshot rollbacks.
- **Universal C-ABI Facade Layer** (Sprint 275) → Projects a normative C-compatible boundary for multi-language execution host environments. Guarantees stateless cross-language invocation bindings for embedded runtime instances.
- **Deterministic State Rewind** (Sprint 276) → Adds native instruction-level time-travel capabilities. Allows agents to trigger state rollbacks to historical execution checkpoints to inspect or correct data topologies before re-executing code lines.
- **Deterministic Execution Path Hashing** (Sprint 277) → VM frames generate cryptographic execution proofs natively. Validates execution path integrity in untrusted host networks without exposing dynamic stack layouts.
- **Cluster-Wide Heterogeneous Work-Stealing** (Sprint 278) → Scales the local work-stealing layout across network topologies. Enables remote direct memory access semantics for horizontal load mitigation between independent cluster runtimes.
- **Adaptive Evolutionary PGO** (Sprint 279) → Connects agent telemetry directly into bytecode execution paths. Dynamically rearranges opcode orders and optimizes active register stack maps at runtime without invocation context teardowns.
- **Sovereign JIT Native Code Generation** (Sprint 280) → Integrates an internal machine code emitter within the JIT runtime. Enables isolates to output native x86_64/ARM64 binary segments directly into executable memory pages for autonomous self-compilation loops.
- **Core Codebase Rectification** (Sprint 281) — Hardens FFI bytecode allocation lifetimes against use-after-free conditions and patch compiler caching anomalies for trigonometric expressions. Establishes isolated evaluation boundaries for global test fixtures.
- **Bytecode Relocation & Telemetry Hardening** (Sprint 282) — Introduces address relocation mechanics for structural AST unrolling. Re-engineers runtime telemetry tracking to use thread-local allocation context boundaries, removing test suite dummy overrides.
- **Executable JIT Memory Pages** (Sprint 283) — Deploys a safe machine code invocation boundary inside native_emit.rs. Converts generated byte vectors into executable native function pointers via memory page mprotect boundaries.
- **Native Branch Emission** (Sprint 284) — Enhances native_emit.rs to transform absolute VM jump opcodes into native machine-level branching patterns, keeping conditional execution fully inside the allocated executable binary memory page.
- **Universal Host SDK Bindings** (Sprint 285) — Introduces idiomatische Python (ctypes) and Node.js (N-API/ffi) boundary shims wrapping the native C-ABI layer to enable embedded runtime multi-language execution orchestration.
- **Distributed Edge WASM Convergence** (Sprint 286) — Compiles the core Stack-VM into highly sandboxed WebAssembly modules. Integrates decentralized WebGPU vertex and compute pipeline dispatches directly over network browser instances.
- **Dynamic Audio Streaming** (Sprint 287) — Migrates audio.rs from synchronous sample pre-allocation to dynamic rodio::Source evaluation. Mitigates command thread starvation and enforces automated idle-sink reclamation.
- **Persistent State Serialization** (Sprint 288) — Deploys robust binary state encoding inside storage.rs. Maps dynamic register stack allocations and execution path hashes into immutable byte-streams for file-system persistence.
- **Distributed Context Migration** (Sprint 289) — Bridges storage serialization with cross-network work-stealing queues. Allows running isolates to halt, hot-serialize, and resume execution contexts on remote cluster destinations seamlessly.
- **Autonomous Orchestration CLI** (Sprint 290) — Integrates a native bootstrapping toolchain inside a new binary target. Standardizes multi-node simulation scaffolding and exports automated verification matrix profiles.
- **Compiler & Scheduler Rectification** (Sprint 291) — Patches native math code emission constraints, introduces native javascript exports for the WASM Edge pipeline via explicit bindgen attributes, and migrates blocking audio streams to real dynamic sources.
- **Architecture Guarding & WASM Interop** (Sprint 292) — Hardens native_emit.rs with target architecture constraints. Fully exposes core VM execution hooks to the browser via verified wasm-bindgen compiler contracts.
- **State Ledger Cryptography** (Sprint 293) — Deploys unmanipulierbare cryptographic chain hashes inside scheduler.rs, enforcing sequencing checks and validating state transitions against historical ledger roots.
- **Isolate Memory Harvesting** (Sprint 294) — Deploys non-blocking registry garbage collection inside machine.rs, evicting dead thread allocations and cleaning migration staging payloads under sub-millisecond thresholds.
- **Autonomous Onboarding Verification** (Sprint 295) — Integrates the 7-task black-box stress-testing protocol to enforce hallucination-free compiler schemas.
- **AST Pipeline Alignment** (Sprint 296) — Aligns node_types.json schemas with VM opcodes. Exposes native AST nodes for standalone compute shaders, isolated actor spawning, and raw chiptune streaming handles without FFI overhead.
- **GUI Telemetry & Inspection** (Sprint 297) — Embeds egui rendering pipelines over active WGPU surfaces to map real-time execution states and cryptographic ledger continuity roots.
- **Distributed Pub-Sub Fabric** (Sprint 298) — Connects abstract bus handles across physical cluster boundaries, mapping transactional stream packets via lock-free peer-to-peer mesh pipelines.
- **Raft Cluster Consensus** (Sprint 299) — Enforces transactional cluster state agreements within scheduler.rs, verifying ledger continuity logs to authorize autonomous cross-node isolate handoffs.
- **Production Stability Calibration** (Sprint 300) — Freezes the normative JSON-AST schema specifications and locks down the multi-node isolate scheduler pipeline for public open-source orchestration.
- **Backend Compilation Sync** (Sprint 301) — Bridges the gap between frontend schema definitions and actual bytecode execution, natively supporting 2D transformation pipelines and non-blocking time throttling.
- **Network Consensus Protocol** (Sprint 302) — Replaced the local deterministic scheduler stub with a true network-layer Raft protocol backing decentralized multi-node state replication.

---

## Extending the Engine (4 Touchpoints)

To add a new native node, update **all four** of these files — no exceptions:

| # | File | What to change |
|---|------|----------------|
| 1 | `knoten_core_types/src/ast.rs` | Add variant to `pub enum Node` |
| 2 | `aether_compiler/src/natives/registry.rs` | Implement native Rust function |
| 3 | `aether_compiler/src/executor.rs` | Add match arm to `evaluate()` |
| 4 | `aether_compiler/src/compiler/codegen.rs` | Add match arm to `generate()` |

After adding a node, also update `aether_compiler/src/validator.rs`, `aether_compiler/src/optimizer.rs`, and `docs/LANGUAGE_REFERENCE/node_types.json`.

---

## Security Sandbox

| Flag | Enables |
|------|---------|
| `--allow-read` | `file_read` (fs module) |
| `--allow-write` | `file_write` (fs module) |
| `--allow-net` | `Fetch`, `net_fetch` |

Unauthorized access returns `ExecResult::Fault` — **never panics**.

---

## Structured Fault Format

Every failure returns:
```rust
ExecResult::Fault { msg: String, node: String }
```
- `msg` — human-readable error description
- `node` — exact origin, e.g. `"Node::MathDiv"`, `"Native::Bridge::net_fetch"`

AI agents: parse `node` first; it pinpoints the failing AST location for immediate self-correction.

---

## Sprint 125: New Comparison & Logic Operators

Six new AST nodes completing the Boolean algebra. All accept `Box<Node>` operands and return `Bool`. They compile to dedicated VM opcodes (`LessEqual`, `GreaterEqual`, `NotEqual`, `And`, `Or`, `Not`).

| Node | Op | Signature | JSON-AST Example |
|------|----|-----------|-----------------|
| `Lte` | `<=` | `Lte(Box<Node>, Box<Node>)` | `{"Lte": [{"Identifier": "x"}, {"IntLiteral": 5}]}` |
| `Gte` | `>=` | `Gte(Box<Node>, Box<Node>)` | `{"Gte": [{"Identifier": "hp"}, {"IntLiteral": 0}]}` |
| `NotEq` | `!=` | `NotEq(Box<Node>, Box<Node>)` | `{"NotEq": [{"Identifier": "mode"}, {"StringLiteral": "off"}]}` |
| `And` | `&&` | `And(Box<Node>, Box<Node>)` | `{"And": [{"BoolLiteral": true}, {"Identifier": "flag"}]}` |
| `Or` | `\|\|` | `Or(Box<Node>, Box<Node>)` | `{"Or": [{"Identifier": "flag"}, {"BoolLiteral": false}]}` |
| `Not` | `!` | `Not(Box<Node>)` | `{"Not": {"Identifier": "active"}}` |

**Usage pattern for agents:**
```json
{"If": [
  {"And": [{"Gte": [{"Identifier": "hp"}, {"IntLiteral": 1}]},
           {"NotEq": [{"Identifier": "state"}, {"StringLiteral": "dead"}]}]},
  {"Block": [{"Print": {"StringLiteral": "alive"}}]},
  null
]}
```

---

## Key Constraints for AI Code Generation

1. **Validate against schema** — `docs/LANGUAGE_REFERENCE/node_types.json` before emitting any node.
2. **No invented keys** — `additionalProperties: false` will reject hallucinated fields at runtime.
3. **Execution Node Routing** — Choose the correct execution path based on this table:

| Scenario | Must Use | Example |
|----------|----------|---------|
| Control flow, math, basic UI | Direct AST Node | `{"If": [...]}` or `{"UITextInput": ...}` |
| Inter-Isolate & user-defined functions | `Call` | `{"Call": ["my_func", [{"IntLiteral": 10}]]}` |
| System FFI (Registry, WGPU, Time, IO) | `ExternCall` | `{"ExternCall": {"module": "registry", "function": "registry_create_window", "args": []}}` |

4. **State-binding pattern** — `text = UITextInput(text)` seeds the buffer on first call; subsequent calls read the live egui buffer.
5. **Never force-push git** — use `git push origin main` only.
6. **Zero warnings policy** — `cargo clippy --workspace --all-targets --all-features -- -D warnings` must produce 0 warnings before any commit; CI enforces this automatically on every push.
7. **Optimize for AOT Native Math (Sprint 128 benchmark)** — The Register Stack-VM demonstrates a **1.21x Native Speedup** vs the JIT path. Prefer native algebraic `Node` operations (`Mul`, `Add`, `Div`) over wrapping intensive computation in FFI `ExternCall` chains. You are a system architect writing instruction streams, not a scripting-layer developer.

---

## Verification & Testing 

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings  # CI Gate 2: 0 warnings
cargo test --workspace --all-features                                  # CI Gate 3: all tests pass
cargo run --bin run_knc -- tests/intentional_crash.knoten
# Expected: Fault: Div by zero (at Node::MathDiv)
```

**Autonomous Testing**: External agents must utilize the [`tests/ai_test_suite/`](tests/ai_test_suite/) directory evaluating strictly controlled structural `.nod` files demonstrating self-healing CLI loop outputs deterministically natively.

---

## Further Reading

| Document | Purpose |
|----------|---------|
| `docs/STDLIB.md` | Standard library modules (`core/math.nod`, `core/time.nod`, etc.) |
| `docs/KNOTEN_SPEC.md` | Full Neural DSL (`.knoten`) language specification |
| `changelog.md` | Sprint history and architectural decisions |
| `CONTRIBUTING.md` | PR checklist and contribution guidelines |
| `SECURITY.md` | Responsible disclosure for sandbox escapes |
| `tools/vscode-knotencore/` | VS Code Language Extension — syntax highlighting & snippets for `.knoten` / `.nod` (Phase 1) |
