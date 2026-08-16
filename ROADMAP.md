# KnotenCore Roadmap

*A high-performance, headless Rust runtime & P2P mesh engine for autonomous AI agents — fully driven by JSON-AST.*

Current engine version: **v2.22.1** · Swarm Phase 2 Completion: Raft Heartbeats & Failure Detection

## Done (selected milestones)

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
