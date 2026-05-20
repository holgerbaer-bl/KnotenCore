# KnotenCore Roadmap

Collected from source-level TODOs and architectural notes extracted during Sprint 176 (The Grand Purge).

## Near-Term

- **VM Garbage Collector**: The `RelType::Dict` uses `Arc<Mutex<HashMap>>` for reference-counted shared objects. A proper GC pass on the VM stack after function returns would reduce orphaned Arc references in long-running scripts.
- **Texture Atlas / Batching**: Currently each textured entity issues a separate draw call with its own bind group. A texture atlas + instanced rendering approach would reduce draw calls for scenes with many entities using the same atlas.
- **Parser Error Routing**: `parser.rs` uses `panic!()` with JSON diagnostic output for parse errors. These should be converted to `Result<Err>` return types so the parser never panics on malformed input, even outside `catch_unwind`.

## Mid-Term

- **Multi-Window Scene Graph**: `KnotenApp` already supports multiple windows, but each has an independent `scene_graph`. A unified scene graph with per-window visibility masks would enable cross-window entity sharing.
- **Compute Shader Backpropagation**: `OpCode::DispatchCompute` submits compute workloads, but there is no mechanism to read back GPU buffer results into the VM stack. Adding `registry_compute_readback` would close the loop.
- **Network Multiplayer RPCs**: `net_fetch` handles basic HTTP requests. A WebSocket-based RPC layer (`net_listen`, `net_send`) would enable real-time multiplayer.

## Far-Term (Speculative)

- **C-ABI FFI Bridge**: The codebase uses pure safe Rust for FFI. A `extern "C"` bridge layer (already guarded by `ffi_safety.rs` validation patterns) would allow loading native `.dll`/`.so` modules from scripts at runtime.
- **Asset Streaming**: Large texture/mesh assets are loaded synchronously. Background streaming with `wgpu::Queue::write_texture` in a dedicated thread would eliminate frame hitches during asset loading.
- **Voxel World Revival**: The isometric software renderer was removed in Sprint 176. A WGPU-accelerated voxel world with greedy meshing and GPU-side octree traversal could replace it.
