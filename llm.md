# KnotenCore — AI Agent Reference (Routing Document) - Sprint 157

> **System Instruction for LLM Code Agents**
>
> You are a **System Architect**, not a game developer. KnotenCore is a **Deterministic AI-Native Execution Runtime** — you feed it structured JSON logic (`Node` objects), and its AOT compiler turns that into flat bytecode executed by a bare-metal Register Stack-VM. Your output is machine instructions, not imperative scripts.
>
> This document is a **routing hub**. Its sole purpose is to direct you to the authoritative
> machine-readable sources in `docs/LANGUAGE_REFERENCE/`. Do **not** invent node names or
> argument shapes. Always validate against the schema before generating code.

---

## 🎯 AI-Readiness Benchmark — Sprint 126

KnotenCore has a **public, reproducible AI-Readiness Benchmark**. If you are an external LLM agent
generating `.nod` programs, your output can be tested against 20 standardised tasks.

**Before you generate any code, read: [`benchmark/README.md`](benchmark/README.md)**

**AG Baseline Score: 20/20 (100%)** — see [`benchmark/results/ag_baseline.md`](benchmark/results/ag_baseline.md)
*Note: As of Sprint 134, all AST Nodes including UI Layouts, file IO, array data operations natively map gracefully in the VM Compiler, and strict audit rectifications confirm 100% crash output parity (`Fault: Div by zero (at Node::MathDiv)`) and heavily sandboxed permissions (`fs_write`, `registry_file_create`). Use AST nodes or FFI Call structures freely based on convenience.*

---

## ⚡ Primary References (Read These First)

| Document | Purpose |
|----------|---------|
| [`docs/LANGUAGE_REFERENCE/node_types.json`](docs/LANGUAGE_REFERENCE/node_types.json) | **Normative JSON Schema** — every AST node, field name, and type constraint. `additionalProperties: false` on all objects. Hallucination-proof. |
| [`docs/LANGUAGE_REFERENCE/nod_grammar.ebnf`](docs/LANGUAGE_REFERENCE/nod_grammar.ebnf) | **Formal EBNF Grammar** — structural grammar of the `.nod` JSON AST. |
| [`docs/LANGUAGE_REFERENCE/native_functions.json`](docs/LANGUAGE_REFERENCE/native_functions.json) | **Native Function Registry** — every FFI function by module, with parameter types, return types, required permissions, and live AST examples. AI agents MUST only call functions listed here. |
| [`docs/LANGUAGE_REFERENCE/examples/99_antipatterns.nod`](docs/LANGUAGE_REFERENCE/examples/99_antipatterns.nod) | **Anti-Pattern Reference** — 10 explicit DO/DON'T patterns for AI code generation. Read before emitting any `.nod` code. |
| [`docs/LANGUAGE_REFERENCE/error_catalog.json`](docs/LANGUAGE_REFERENCE/error_catalog.json) | **Error Catalog** — registry of execution fault codes and self-healing hints for AI agents. |
| [`docs/KNOTEN_SPEC.md`](docs/KNOTEN_SPEC.md) | **KNOTEN_SPEC.md** — Human-readable language reference, derived from `src/ast.rs`. If spec and source diverge, `src/ast.rs` wins. |
| [`src/ast.rs`](src/ast.rs) | **Rust source of truth** — `pub enum Node` is the canonical definition. If schema and source diverge, source wins. |

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
- **All OS I/O** → sandboxed; permissions must be granted via CLI flags (`--allow-read`, `--allow-write`, `--allow-net`)
- **Language Server (LSP)** → `knoten_lsp` binary validates `.nod` JSON documents in real-time. The **VS Code Extension** automatically launches this server, flagging unknown opcodes (`ERR_UNKNOWN_NODE`) and JSON parse errors (`ERR_JSON_PARSE`) directly in the editor before they reach the runtime. Tracing output is visible in the VS Code *Output → knoten-lsp* channel.
- **GitHub Linguist** → `.nod` targets `JSON` and `.knoten` targets `JavaScript` for correct repository rendering.

---

## Extending the Engine (4 Touchpoints)

To add a new native node, update **all four** of these files — no exceptions:

| # | File | What to change |
|---|------|----------------|
| 1 | `src/ast.rs` | Add variant to `pub enum Node` |
| 2 | `src/natives/registry.rs` | Implement native Rust function |
| 3 | `src/executor.rs` | Add match arm to `evaluate()` |
| 4 | `src/compiler/codegen.rs` | Add match arm to `generate()` |

After adding a node, also update `validator.rs`, `optimizer.rs`, and `docs/LANGUAGE_REFERENCE/node_types.json`.

---

## Security Sandbox

| Flag | Enables |
|------|---------|
| `--allow-read` | `FSRead`, `FileRead`, `registry_read_file`, `registry_texture_load` |
| `--allow-write` | `FSWrite`, `FileWrite`, `registry_write_file`, `registry_file_create` |
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
| Legacy FFI functions | `NativeCall` (Deprecated) | `{"NativeCall": ["print", ...]}` |
| All modern native FFI / I/O | `ExternCall` | `{"ExternCall": {"module": "registry", "function": "registry_create_window", "args": []}}` |

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
