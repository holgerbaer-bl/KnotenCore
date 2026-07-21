# KnotenCore Roadmap

Current engine version: **v2.3.2-hotfix** · Sprint 305 · 247/247 tests · 0 Clippy warnings

## Done (selected milestones)

- ✅ Stack-VM with AOT compiler + JIT evaluator
- ✅ Full sandbox (FS, network whitelist, symlink blocking, watchdog, 1M opcode cap, 16MB memory guard)
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
- ✅ Raft consensus (leader election, quorum commit, failover)
- ✅ WASM-bindgen CI gates
- ✅ C-ABI facade + Python/Node.js bindings
- ✅ Language Server (tower-lsp) with real-time diagnostics
- ✅ AI-Readiness Score 20/20 — external LLMs generate correct `.nod` without human correction
- ✅ Compute readback (previously Near-Term)
- ✅ Texture atlas / instanced rendering groundwork
- ✅ `machine.rs` modularisation (split into `vm_core`, `gpgpu`, `inspector`, `ledger` in Sprint 303)
- ✅ Parser panics → `Result` / JSON-feedback validation (Sprint 304)
- ✅ Headless Engine Transition & Sandbox Wächter Hardening (Sprint 305)

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
