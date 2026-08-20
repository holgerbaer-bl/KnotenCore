# KnotenCore Roadmap

*A high-performance, headless Rust runtime & P2P mesh engine for autonomous AI agents — fully driven by JSON-AST.*

Current engine version: **v2.24.16** · CRDT State Digests & Differential Mesh Sync

## Done (selected milestones)

- ✅ CRDT State Digests & Differential Mesh Sync (deterministic `ring::digest::SHA256` anti-entropy state digests `knc_store_digest`, differential sync `knc_store_diff`, LWW delta merging, documented writer_id trust boundary limitation, Section 7.15 of `docs/KNOTEN_SPEC.md`, Sprint 353 - v2.24.16)
- ✅ Zero-Trust Raft Heartbeats & Anti-Downgrade Hardening (deprecated plaintext auth tokens in Raft governance heartbeats, canonical Ed25519 signature verification, peer revocation filtering, anti-downgrade enforcement, Section 7.14 of `docs/KNOTEN_SPEC.md`, Sprint 352 - v2.24.15)
- ✅ WebSocket Robustness, Gossip Revocation Gate & Test Realignment (hardened WebSocket frame handling against `try_clone` failure, strict revocation filter in `knc_mesh_peers?action=gossip`, eviction of incoming revoked peers, realigned test badge to 280/280 verified tests, Section 7.13 of `docs/KNOTEN_SPEC.md`, Sprint 351 - v2.24.14)
- ✅ Zero-Trust Host vs. Guest Architecture & Agent Orchestration Guide (standardized inline guest demarcation headers in `examples/03_agents_and_zero_trust/`, comprehensive host orchestration guide `examples/03_agents_and_zero_trust/README.md`, CLI/JSON-RPC signed payload submission examples, host Rust code strictly using `aether_compiler::crypto_ed25519` `ring` module, Section 7.12 of `docs/KNOTEN_SPEC.md`, Sprint 350 - v2.24.13)
- ✅ CI Workflow Hardening & Non-Interactive Release Builds (hardened GitHub Actions `ci.yml` and `release.yml` against interactive terminal hangs, `DEBIAN_FRONTEND: noninteractive`, `needrestart` prompt suppression via silent configuration, 10-minute job execution boundaries across Linux, Windows, macOS runners, Section 7.11 of `docs/KNOTEN_SPEC.md`, Sprint 349 - v2.24.12)
- ✅ Dogfooding & Real-World Mesh Examples (real-world Stage-2 agent orchestration `task_offloading.knoten` and `mesh_telemetry.knoten`, Stage-1 SIMD vector math utility `vector_math.knoten`, Tier 1 end-to-end VM evaluation and Tier 2 compilation integrity verification, Section 7.10 of `docs/KNOTEN_SPEC.md`, Sprint 348 - v2.24.11)
- ✅ CI Examples Verification Harness (dedicated two-tier automated integration test suite `tests/examples_verification.rs`, recursive `.knoten` script discovery under `examples/`, Tier 1 end-to-end VM runtime execution for getting started & compute scripts, Tier 2 syntax & bytecode compilation integrity for zero-trust sandbox & egui UI scripts, dynamic subfolder safeguards, Sprint 347 - v2.24.10)
- ✅ Examples Directory Cleanup & Restructuring (standardized `.knoten` script extensions across all examples, purged obsolete test relics and legacy `.nod` files, restructured `examples/` workspace into 4 thematic categories: `01_getting_started`, `02_vector_and_compute`, `03_agents_and_zero_trust`, `04_interactive_and_ui`, aligned `test_examples_compilation_and_validation`, Sprint 346 - v2.24.9)
- ✅ Zero-Trust P2P Mesh Gossip Protocol & Cryptographic Task Offloading (epidemic gossip discovery and peer load telemetry `GossipState` / `PeerMetrics`, latency-weighted/load-aware peer selection, Ed25519-signed `GossipFrame` transport, cryptographic task payload and worker result verification `SignedTaskResult`, per-peer rate-limiting `MAX_PER_PEER_TASK_RATE`, queue flood protection `MAX_TASK_QUEUE_DEPTH`, zero-trust sandboxed remote execution, Sprint 345 - v2.24.8)
- ✅ Vector Gas Metering Fix, CI Formatting Rectification & Benchmark Spec (strict vector opcode gas error propagation `self.gas_meter.consume(...)?`, execution ordering hardening, `cargo fmt` formatting rectification in `bench.rs`, authentic release benchmark measurements in `docs/BENCHMARKS.md`, explicit RPC authentication semantics documentation, Sprint 344.1 - v2.24.7)
- ✅ SIMD Vector Compute Engine, Batch OpCodes & Matrix Benchmarks (contiguous numeric buffer representations `Vec<f64>` / `Vec<i64>`, SIMD-accelerated batch opcodes `VectorDot`, `VectorAdd`, `VectorMul`, AST vector lowering, element-proportional batch gas accounting, formal vector benchmark workload `VectorDotProduct(100_000)`, Sprint 344 - v2.24.6)
- ✅ Zero-Trust RPC Mesh Auth Hardening & Dynamic Introspection (`knc_eval_isolate` auth guard `-32001`, `RpcServer::registered_methods()`, dynamic reflection-safe auth compliance test suite, `use aether_compiler::vm::machine::{VM, VMError};` rustfmt alignment, Sprint 343.1 - v2.24.5)
- ✅ Isolate Gas Metering, Execution Watchdog & Resource Quotas (instruction gas metering `GasMeter`, microsecond wall-clock watchdog, hard memory heap allocation boundaries, `VMError::GasExhausted`, `VMError::MemoryQuotaExceeded`, `VMError::WatchdogTimeout`, RPC `knc_eval_isolate`, Sprint 343 - v2.24.4)
- ✅ Deep Thought 42 Intrinsic & Deterministic Protocol Extension (built-in `knc_meaning_of_life` intrinsic & RPC endpoint returning Hitchhiker payload metadata across Evaluator and VM engines with full parity, Sprint 342 - v2.24.3)
- ✅ AI Agent Ecosystem & Safe PR Feedback Workflow (AI agent directives manifest `AI.md`, standardized bot report issue template `.github/ISSUE_TEMPLATE/bot_report.md`, PR diagnostic feedback workflow `docs/workflows/agent-ci-feedback.yml`, README badges, strict human maintainer review invariant, Sprint 341 - v2.24.2)
- ✅ Benchmark Engine Rectification & Real Tree-Walking Interpreter Comparison (AST Tree-Walking Interpreter vs. AOT Bytecode Stack-VM on identical AST inputs, deterministic result parity validation, authentic AOT speedup calculation, Sprint 340 - v2.24.1)
- ✅ Formal Benchmark Suite & English Standardization (formal benchmark engine `BenchmarkEngine`, `knoten bench` CLI harness, comprehensive RPC handler re-exports, 100% English documentation standardization, dedicated benchmark specification `docs/BENCHMARKS.md`, Sprint 339 - v2.24.0)
- ✅ Architectural Modularization & Codebase Detox (submodule refactoring of monolithic `rpc.rs` into `aether_compiler/src/rpc/`, state consolidation, removal of historical Sprint/Prompt tags, 100% backward-compatible re-exports and API dispatching across all 28 JSON-RPC endpoints, integration test suite `tests/rpc_modularization_tests.rs`, Sprint 338 - v2.23.1)
- ✅ Scoped Hot-Module-Replacement (`knc_isolate_reload` 28th endpoint, `VMIsolate::hot_reload_code`, transactional pre-compilation validation, execution scoping against stack corruption during active execution `ERR_HMR_ACTIVE_EXECUTION`, environment/heap/quota state preservation, integration test suite `tests/isolate_hmr_tests.rs`, Sprint 337 - v2.23.0)
- ✅ Swarm Phase 2 Completion: Raft Heartbeats & Failure Detection (`knc_swarm_heartbeat` 27th endpoint, Leader background heartbeat loop with lock hygiene, follower/worker failure detection with randomized 300–500 ms timeout and automatic re-election, multi-node TCP integration tests, Sprint 336 - v2.22.1)
- ✅ Swarm Phase 2: Distributed Raft Voting & Consensus (`knc_swarm_request_vote` 26th endpoint, term-tracking & single-vote-per-term invariant, mandatory mesh auth-gating against term inflation, dynamic election broadcast with strict lock hygiene, majority quorum decision `votes_count > active_nodes / 2`, randomized backoff sleep 150–300 ms, multi-node TCP cluster integration suite, Sprint 335 - v2.22.0)
- ✅ Audit Completion & State Persistence (persistent peer revocation storage `revoked_keys.json`, peer registration gate against revoked keys, active-peer quorum denominator calculation in `knc_swarm_quorum` and `knc_mesh_revoke_peer`, stack heap memory estimation fix, custom `IsolateQuota` support in `VMIsolate`, Sprint 334 - v2.21.4-security)
- ✅ CI Test Isolation & Swarm Expectation Reconciliation (removal of `cfg!(test)` runtime bypass in `SwarmGovernance::elect()`, isolation of test fixtures, addition of `#[cfg(test)]` reset helper, reconciled election expectations, Sprint 333 - v2.21.3-security)
- ✅ Root Election Hardening & Exhaustive String Bounds (complete blocking of unilateral self-nomination after bootstrap in `SwarmGovernance::elect()`, removal of client bypass params, exhaustive `validate_param_string_len` enforcement across all `session_id` and `nonce_str` extractions, Sprint 332 - v2.21.2-security)
- ✅ Security Audit Rectification & Resource Limits (`MAX_BODY_BYTES` & `MAX_WS_PAYLOAD` 1 MiB caps, HMAC Nonce-LRU replay defense, `MAX_PARAM_STRING_LEN` 256B cap, `MAX_CALL_DEPTH` 512 frame guard, self-election locks, Sprint 331 - v2.21.1-security)
- ✅ Repository Documentation Overhaul & Specification Realignment (Tabula Rasa overhaul of `CONTRIBUTING.md`, `README.md`, `llm.md`, `ROADMAP.md`, `docs/KNOTEN_SPEC.md`, Sprint 330 - v2.21.0-authz)
- ✅ Server-Enforced Swarm Quorum & Quorum-Gated Peer Revocation (`(active_nodes / 2) + 1` quorum enforcement in `knc_swarm_quorum`, strict prohibition of `force: true` self-election in Zero-Trust mode, quorum-consensus gated peer revocation in `knc_mesh_revoke_peer`, Sprint 329 - v2.21.0-authz)
- ✅ Comprehensive RPC Auth Bypass Mitigation & Snapshot/Restore Hardening (`check_mesh_auth` enforced across ALL 25 JSON-RPC endpoints, Hotfix - v2.20.1-security)
- ✅ Zero-Trust Mesh Phase 2: Ed25519 Key Rotation, Nonce LRU Eviction & Peer Revocation (`knc_mesh_rotate_key`, `knc_mesh_revoke_peer`, `NonceCache` LRU eviction, keyless session migration, Sprint 328 - v2.20.0-trust)
- ✅ Zero-Trust Mesh Phase 1: Cryptographic Envelope Signing & Replay Protection (`knc_mesh_verify_peer`, Ed25519 envelope signing, 30s sliding replay window, anti-downgrade guard, Sprint 327 - v2.19.0)

- ✅ Stack-VM with AOT compiler + JIT evaluator
- ✅ Full sandbox (FS, network whitelist, symlink blocking, watchdog, 1M opcode cap, 16MB memory guard)
- ✅ Headless-First Feature-Gate Refactoring & Core Decoupling (`--features ui`, v2.12.0)
- ✅ Agentic Mesh Protocol & Inter-Node Snapshot Teleportation (`knc_mesh_discover`, `knc_mesh_peers`, `knc_agent_teleport`, v2.13.0)
- ✅ Mesh Peer Gossip Protocol, Heartbeats & Auto-Healing Eviction (Sprint 317 - v2.14.0)
- ✅ Deep Security Audit & HMAC Mesh Hardening — official release (Sprint 318 - v2.14.1)
- ✅ Distributed Task Queue & Mesh Work-Stealing Engine (`knc_task_submit`, `knc_task_status`, `knc_task_cancel`, `knc_task_steal`, Sprint 319 - v2.15.0-task)
- ✅ Cluster Metrics & Adaptive Work-Stealing Protocol (`knc_mesh_metrics`, overload guard CPU >80%, Sprint 320 - v2.16.0-metrics)
- ✅ Distributed CRDT Key-Value Storage & State Sync (`knc_store_put`, `knc_store_get`, `knc_store_sync`, LWW-Set, Sprint 321 - v2.17.0-store)
- ✅ Security Audit Rectification & Deep Hardening — official release (CRDT Timestamp Drift Guard, Task Queue Limits & GC, Replay Window Validation, Store & Sync Bounds, v2.17.1)
- ✅ Swarm Governance & Grounded Documentation — official release (`knc_swarm_elect`, `knc_swarm_roles`, `knc_swarm_quorum`, `NodeRole` Leader/Worker/Storage/Observer topology, v2.18.0)
- ✅ Swarm Governance Terminology Refinement & Grounding (Local Swarm Role Management & Leadership Claim Primitives Phase 1, `knc_swarm_elect` local state management, Sprint 324 - v2.18.1-swarm)
- ✅ Wire Capability Renaming & Systemic Terminology Sweep — official release (`swarm_leadership` capability, systemic sweep, v2.18.1)
- ✅ Headless engine transition with optional `ui` feature gate (WGPU / Winit / Egui)
- ✅ WGPU retained-mode scene graph + egui UI
- ✅ Polyphonic audio synth (Sine/Sawtooth/Square/Triangle + ADSR)
- ✅ GPGPU compute with lock-free crossbeam readback
- ✅ SIMD matrix algebra via glam
- ✅ x86_64 JIT native code emission (memmap2, no LLVM)
- ✅ Adaptive PGO loop unrolling
- ✅ Cryptographic state ledger (SHA-256 chain, replay-attack defense)
- ✅ Multi-threaded VM Isolates + work-stealing scheduler
- ✅ Cross-node isolate migration (binary snapshot → network → resume)
- ✅ P2P Mesh-Bus distributed pub/sub
- ✅ In-memory Raft consensus simulator & scheduler harness (Sprint 301)
- ✅ WASM-bindgen CI gates
- ✅ C-ABI facade + Python/Node.js bindings
- ✅ Language Server (tower-lsp) with real-time diagnostics
- ✅ AI-Readiness Score 20/20 — external LLMs generate correct `.nod` without human correction
- ✅ Compute readback (previously Near-Term)
- ✅ Texture atlas / instanced rendering groundwork
- ✅ `machine.rs` modularisation (split into `vm_core`, `gpgpu`, `inspector`, `ledger` in Sprint 303)
- ✅ Parser panics → `Result` / JSON-feedback validation (Sprint 304)
- ✅ Headless Engine Transition & Sandbox Wächter Hardening (Sprint 305)
- ✅ Native Cast Opcodes, High-Performance String & Array Primitives, Sandboxed In-Memory VFS (Sprint 306 - v2.4.0-core)
- ✅ Constant Folding for Casts, String Primitives & Unreachable Code Trimming (Sprint 307 - v2.5.0-opt)
- ✅ Agentic Event Streaming Hooks & EventEmit Opcode (Sprint 308 - v2.6.0-event)
- ✅ Async Yield Opcode, Non-blocking VM Suspension & Resuming (Sprint 309 - v2.7.0-async)
- ✅ Headless JSON-RPC 2.0 Server Interface & Agentic Transport Protocol (Sprint 310 - v2.8.0-rpc)
- ✅ Isolate Multi-Tenant Resource Quotas & RPC Session Enforcement (Sprint 311 - v2.9.0-isolate)
- ✅ Persistent WebSocket RPC Transport & Realtime Event Broadcaster (Sprint 312 - v2.10.0-ws)
- ✅ Agentic Execution Protocol, Portable State Snapshots & Restore (Sprint 313 - v2.11.0-agent)
- ✅ Comprehensive Documentation Alignment & Workspace Consolidation (Sprint 314 - v2.11.1-docs)
- ✅ Security & Architecture Audit Fixes (v2.11.2-audit)
- ✅ Headless-First Feature-Gate Refactoring & Core Decoupling (Sprint 315 - v2.12.0-core)
- ✅ Agentic Mesh Protocol & Inter-Node Snapshot Teleportation (Sprint 316 - v2.13.0)

## Near-Term

- **`ROADMAP.md` auto-update hook**: Sprint changelog entries should diff into the roadmap automatically to keep them in sync.

## Mid-Term

- **WebSocket RPC** (`net_listen` / `net_send`): `net_fetch` handles one-shot HTTP. A persistent WebSocket layer would enable real-time multi-node messaging and agent-to-agent RPC without polling.
- **Multi-Window Scene Graph**: `KnotenApp` supports multiple windows, each with an independent `scene_graph`. A unified scene graph with per-window visibility masks would enable cross-window entity sharing.
- **Hot-Module-Replacement**: Extend the existing hot-swap registry so agents can push new bytecode into *running* isolates without a restart — live code reload mid-execution.
- **Formal benchmark suite**: Compare AOT Stack-VM against V8, Lua 5.4, and Wren on equivalent algorithmic workloads. Publishable numbers for the `knotencore.de` engineering page.

## Far-Term (Speculative)

- **Voxel World Revival**: A WGPU-accelerated voxel world with greedy meshing and GPU-side octree traversal (the isometric software renderer was removed in Sprint 176).
- **WASM Edge Mesh**: Scale isolates across browser nodes via the existing `wasm_edge.rs` foundation — connect peer runtimes via WebRTC data channels.
- **Knoten Package Registry**: A hosted registry for `.nod` programs and native extensions, analogous to crates.io, but for KnotenCore agents.
- **Tier-2 JIT targets**: ARM64 native emission (currently x86_64 only). Non-x86_64 hosts fall back to the software interpreter; native ARM64 emission would close the gap.
