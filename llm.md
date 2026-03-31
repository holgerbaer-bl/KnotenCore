# KnotenCore — AI Agent Reference (Routing Document)

> **System Instruction for LLM Code Agents**
>
> This document is now a **routing hub**. Its sole purpose is to direct you to the authoritative
> machine-readable sources in `docs/LANGUAGE_REFERENCE/`. Do **not** invent node names or
> argument shapes. Always validate against the schema before generating code.

---

## ⚡ Primary References (Read These First)

| Document | Purpose |
|----------|---------|
| [`docs/LANGUAGE_REFERENCE/node_types.json`](docs/LANGUAGE_REFERENCE/node_types.json) | **Normative JSON Schema** — every AST node, field name, and type constraint. `additionalProperties: false` on all objects. Hallucination-proof. |
| [`docs/LANGUAGE_REFERENCE/nod_grammar.ebnf`](docs/LANGUAGE_REFERENCE/nod_grammar.ebnf) | **Formal EBNF Grammar** — structural grammar of the `.nod` JSON AST. |
| [`docs/LANGUAGE_REFERENCE/native_functions.json`](docs/LANGUAGE_REFERENCE/native_functions.json) | **Native Function Registry** — every FFI function by module, with parameter types, return types, required permissions, and live AST examples. AI agents MUST only call functions listed here. |
| [`docs/LANGUAGE_REFERENCE/examples/99_antipatterns.nod`](docs/LANGUAGE_REFERENCE/examples/99_antipatterns.nod) | **Anti-Pattern Reference** — 10 explicit DO/DON'T patterns for AI code generation. Read before emitting any `.nod` code. |
| [`docs/LANGUAGE_REFERENCE/error_catalog.json`](docs/LANGUAGE_REFERENCE/error_catalog.json) | **Error Catalog** — registry of execution fault codes and self-healing hints for AI agents. |
| [`src/ast.rs`](src/ast.rs) | **Rust source of truth** — `pub enum Node` is the canonical definition. If schema and source diverge, source wins. |

---

## Architecture in 60 Seconds

KnotenCore is a **hybrid JIT/AOT engine** written in Rust:

```
.nod JSON  →  Parser  →  AST (Node enum)  →  Executor (JIT)
                                          ↘  VM Compiler   →  Bytecode  →  Stack Machine (AOT)
```

- **High-level UI / side-effects** → always executed by the **JIT Executor** (`src/executor.rs`)
- **Pure math / computation loops** → compiled to flat **Opcodes** and run by the **Register VM** (`src/vm/`)
- **egui UI** → rendered per-frame over WGPU; UI nodes emit `RenderCommand` messages across a channel to `src/window.rs`
- **All OS I/O** → sandboxed; permissions must be granted via CLI flags (`--allow-read`, `--allow-write`, `--allow-net`)

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
6. **Zero warnings policy** — `cargo clippy --lib` must produce 0 warnings before any commit.

---

## Verification

```bash
cargo clippy --lib           # 0 warnings required
cargo test                   # all 55+ integration tests must pass
cargo run --bin run_knc -- tests/intentional_crash.knoten
# Expected: Fault: Div by zero (at Node::MathDiv)
```

---

## Further Reading

| Document | Purpose |
|----------|---------|
| `docs/STDLIB.md` | Standard library modules (`core/math.nod`, `core/time.nod`, etc.) |
| `docs/KNOTEN_SPEC.md` | Full Neural DSL (`.knoten`) language specification |
| `changelog.md` | Sprint history and architectural decisions |
| `CONTRIBUTING.md` | PR checklist and contribution guidelines |
| `SECURITY.md` | Responsible disclosure for sandbox escapes |
