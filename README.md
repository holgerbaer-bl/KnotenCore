# KnotenCore

KnotenCore is a Rust-based AOT compiler and JIT runtime for JSON-encoded Abstract Syntax Trees, featuring a deterministic ARC resource registry.

[![Rust](https://img.shields.io/badge/Language-Rust-orange)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

---

## Capabilities

| Feature | Description |
|---|---|
| **JSON-AST Language** | Programs are structured JSON data. |
| **Compiler & Runtime** | Ahead-of-Time compilation to Rust or instant Execution via JIT. |
| **Memory Safe OS Handles** | Automatic Resource Counting internally cleans Native File System handles unconditionally on drop limits. |
| **Static Type Inference** | Evaluated boundaries enforce Type logic mathematically. |
| **AST Optimizer** | Constant folding trims JSON bounds prior to evaluations. |
| **WGPU & Audio Engines** | Included rendering tools mapping graphics endpoints natively via FFI. |

---

## Development Milestones

- **Sprint 40**: Transpiler - Code generation from valid AST structs dynamically. Unrolls nested `Block` loops into structured Rust `output.rs` files enforcing Scope drops.
- **Sprint 41**: Native IO Bridge (File System Orchestration) - Strict OS memory handles managed via Registry protocols securely exposing `std::fs::File` persistence limits implicitly. 

---

## Quickstart

```bash
# Clone
git clone https://github.com/holgerbaer-bl/KnotenCore.git
cd KnotenCore

# Run the v0.1.0 Showcase Demo natively
cargo run --bin run_knc examples/core/showcase_v1.nod

# Transpile into Rust manually
cargo run --bin run_knc --transpile examples/io/file_magic.nod
rustc output.rs
.\output.exe
```

## How It Works

KnotenCore evaluates AST structs structurally directly matching Rust paradigms:

```json
{
  "Block": [
    { "Assign": ["my_file", { "Call": ["registry_file_create", [{ "StringLiteral": "knoten_test.txt" }]] }] },
    { "Call": ["registry_file_write", [{ "Identifier": "my_file" }, { "StringLiteral": "KnotenCore Execution Success!" }]] }
  ]
}
```

The underlying memory bounds are guaranteed structurally. The engine evaluates JSON branches gracefully tracking state boundaries.

---

## Repository Structure

```
src/
├── ast.rs              # AST node definitions & Type enum
├── executor.rs         # JIT evaluation engine
├── compiler/           # Transpiler bounds mapping code generation
├── optimizer.rs        # Constant folding
├── validator.rs        # Structural integrity checks
├── lib.rs              # Crate boundaries
├── bin/
│   ├── run_knc.rs      # Native binary compilation router
│   └── rust_ingest.rs  # Legacy bindings router
└── natives/            # Engine FFI endpoints bridging standard libs natively
    ├── math.rs         
    ├── io.rs           
    ├── registry.rs     # Strict handles memory Mutex structs natively
    └── bridge.rs       

examples/               # Scripts resolving endpoints natively
docs/
tests/                  # Rust compiler testing checks
stdlib/                 # KnotenCore JSON modules mapping Rust traits
```

## Prerequisites

- [Rust](https://www.rust-lang.org/) (Latest Stable)
- Compatible Vulkan/DX12 GPU (Required strictly for UI engine testing parameters)
