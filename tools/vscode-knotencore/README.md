# KnotenCore — VS Code Language Support ⚡

[![Version](https://img.shields.io/badge/version-0.1.0-blue)](https://github.com/holgerbaer-bl/KnotenCore)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![CI Status](https://github.com/holgerbaer-bl/KnotenCore/actions/workflows/ci.yml/badge.svg)](https://github.com/holgerbaer-bl/KnotenCore/actions)

Provides **advanced language support** for the KnotenCore AI-Native Execution Runtime. This extension bridges the gap between the bare-metal engine and your IDE, offering a premium development experience for both human architects and autonomous agents.

## 🚀 Features

### 🔌 Language Server (LSP) — Phase 2 & 3
The native `knoten_lsp` server provides deep intelligence for `.nod` (AST) and `.knoten` (DSL) files:
- **Real-time Diagnostics**: Catches `ERR_UNKNOWN_NODE` (hallucinated nodes) and `ERR_JSON_PARSE` immediately.
- **Hover Documentation**: Hover over any `registry_*` or `Call` function to see full Markdown documentation, parameters, and return types sourced from the official engine registry.
- **Intelligent Auto-completion**: Context-aware suggestions for all OpCodes and native FFI functions.
- **Structural Tracing**: View server lifecycle and diagnostics in the *Output → knoten-lsp* channel.

### 🎨 Semantic Highlighting
- **Knoten DSL**: Full grammar for the neural scripting language, including hex literals, bitwise operators, and UI layout primitives.
- **AST Nodes**: High-contrast highlighting for the full instruction set (`If`, `While`, `Assign`, `Constant`, etc.).
- **Native FFI**: Specialized colors for namespaces like `registry.*`, `ui.*`, `fs.*`, and `sys.*`.

### ⚡ Developer Productivity
- **Code Snippets**: Instant boilerplate for window management, raycasting, and UI layout patterns (try `kc-window` or `kc-uiwindow`).
- **Bracket Matching**: Intelligent pairing for `{}`, `[]`, and `()`.

## 📂 Supported File Types

| Extension | Language ID | Description |
|-----------|-------------|-------------|
| `.knoten` | `knoten` | KnotenCore Neural DSL — Human-readable scripting |
| `.nod`    | `nod`     | KnotenCore JSON-AST — Machine-executable instruction streams |

## 🛠️ Installation

**3-Step Setup for Full Language Support:**

### Step 1: Install Extension
Install directly from the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=holger-bl.knotencore) or manually via `.vsix`:
`code --install-extension holger-bl.knotencore-1.0.0.vsix`

### Step 2: Clone & Build Engine
The extension requires the `knoten_lsp` Language Server to provide advanced features.
```bash
git clone https://github.com/holgerbaer-bl/KnotenCore.git
cd KnotenCore/aether_compiler
cargo build --release --bin knoten_lsp
```

### Step 3: Run & Configure
The extension automatically searches your workspace (and `PATH`) for the compiled `knoten_lsp` binary. Alternatively, you can explicitly set the path in your VS Code settings:
`"knotencore.lspPath": "C:/path/to/knoten_lsp.exe"`

## 🛡️ License
Distributed under the **MIT License**. See `LICENSE` in the repository root for more information.

---
**KnotenCore** — *The Deterministic AI-Native Execution Runtime.*

