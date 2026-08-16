# AGENT_VALIDATION_REPORT — Sprint 295

## Executive Summary
The external AI agent (DeepSeek V4 Pro) was evaluated using a 7-task black-box protocol against documented specifications (`llm.md`, `node_types.json`, `native_functions.json`, `error_catalog.json`). Out of 7 tasks, 4 were fully resolved and 3 were partially resolved (Score: 4/7). Primary limitations: GPGPU shader synthesis and Isolate RPC are only accessible via the OpCode compiler (not the AST parser). Audio synthesis requires the `registry_play_tone_panned` native function with 5 arguments, which was correctly identified. Zero hallucinations (fictional node types or functions) were produced. Documentation is sufficiently precise for an AI agent, though AST-to-OpCode mapping documents are missing for advanced features.

## Task Results Matrix

| Task | Iterations | Initial Error | Self-Resolved | Status |
|------|------------|---------------|---------------|--------|
| 1 — Arithmetic Loop | 3 | `While` expected Bool node (not IntLiteral 200) | Yes | **PASS** |
| 2 — Data Structure (JSON) | 2 | `EvalJSONNative` identified, `json_parse` as alternative | Yes | **PASS** |
| 3 — Isolate RPC | 4 | `spawn_isolate` available only as OpCode/FFI, no AST node | No | **PARTIAL** |
| 4 — Error Handling (Div/0) | 1 | Division by zero as OpCode, fault detection in VM | Yes | **PASS** |
| 5 — GPGPU Compute | 3 | `DispatchCompute`/`LoadComputeShader` available as OpCode, no AST node | No | **PARTIAL** |
| 6 — Audio Synth | 2 | `registry_play_tone_panned` requires 5 args | Yes | **PASS** |
| 7 — Combined Pipeline | 3 | GPGPU path not accessible via AST | No | **PARTIAL** |

## Hallucination Protocol
- **Zero Hallucinations.** The agent used no invented node types, parameters, or functions. All utilized types are documented in `node_types.json`, `native_functions.json`, or `llm.md`.

## Task Details

### Task 1 — Arithmetic Loop (PASS)
Iteration 1: `{"While": [{"IntLiteral": 200}, ...]}` → `While` expects Bool, not Int.  
Iteration 2: `{"While": [{"Lt": [{"Identifier": "i"}, {"IntLiteral": 200}]}, ...]}` → correct.  
Iteration 3: Managed variables `sum` + `i` using `Assign`. Sum of all even numbers = 10100.

### Task 2 — Data Structure (PASS)
Iteration 1: Identified `json_parse` + `file_read`. `json_parse` accepts String.  
Functional path: `file_read("config.json") → json_parse → extract → calculate → file_write`.

### Task 3 — Isolate RPC (PARTIAL)
Isolate spawning exists only as a C-ABI function (`knotencore_spawn_isolate`) and as an OpCode in the VM compiler. There is no AST node `SpawnIsolate` or `MailboxSend` in `node_types.json`. The agent correctly identified and documented this constraint. Workaround: `ExternCall` to C-ABI is not available in the pure AST parser.

### Task 4 — Error Handling (PASS)
Division by zero: `{"Div": [{"IntLiteral": 1}, {"IntLiteral": 0}]}`. Compiler catches this as `Fault: Div by zero`. Correctly expressible at AST level.

### Task 5 — GPGPU Compute (PARTIAL)
`LoadComputeShader` and `DispatchCompute` exist only as OpCodes, not as AST nodes. The agent correctly identified that `DispatchComputeLoop` is an OpCode. Shader synthesis is accessible only via JIT compilation (`shader_graph.rs`), not via the `.nod` parser.

### Task 6 — Audio Synth (PASS)
Identified `registry_play_tone_panned`: `channel` (Int), `frequency` (Float), `duration_ms` (Int), `waveform` (Int: 0=Sine), `pan` (Float). C4-C5 scale with 13 semitone steps via repeated `ExternCall`.

### Task 7 — Combined Pipeline (PARTIAL)
JSON config reading + conditional branching functional. GPGPU path via AST not reachable (see Task 5). CPU loop path functional. Log writing via `file_write` functional. Isolate encapsulation not reachable via AST (see Task 3).

## AI-Readiness Score: 4/7

### Functional Areas (AI-Friendly):
- Arithmetic, Control Flow (If, While), Variables via AST
- File I/O with permissions
- JSON Parsing and Stringify
- Audio synthesis via `ExternCall`
- Error handling (Fault Catching)

### Documentation & AST Coverage Gaps:
- GPGPU Compute Shader: OpCode only, no AST node
- Isolate Spawning: C-ABI / OpCode only, no AST node
- JIT Compilation: No AST access
- `PlayNote` / `StopNote` AST nodes: Not in `node_types.json`, OpCode only

### Recommendations:
1. Extend `node_types.json` with `DispatchCompute` and `LoadComputeShader` AST nodes.
2. Document `registry_play_tone` in `native_functions.json` (without `_panned`) with default `pan = 0.0`.
3. Expose Isolate API as an AST node or documented `ExternCall` path.
