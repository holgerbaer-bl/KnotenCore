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

### Marketplace (Recommended)
Install directly from the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=knotencore.vscode-knotencore).

### Manual Installation
1. Download the latest `.vsix` from the [Releases](https://github.com/holgerbaer-bl/KnotenCore/releases) page.
2. Run: `code --install-extension vscode-knotencore-0.1.0.vsix`

## ⚙️ Configuration
The extension automatically detects the `knoten_lsp` binary in your workspace `target` folders. To use a custom server path, ensure `knoten_lsp` is in your system `PATH`.

## 🛡️ License
Distributed under the **MIT License**. See `LICENSE` in the repository root for more information.

---
**KnotenCore** — *The Deterministic AI-Native Execution Runtime.*

