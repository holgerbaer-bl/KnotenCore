# KnotenCore — VS Code Language Extension

Provides **syntax highlighting**, **bracket matching**, and **code snippets** for KnotenCore source files in Visual Studio Code.

## Supported File Types

| Extension | Language ID | Description |
|-----------|-------------|-------------|
| `.knoten` | `knoten` | KnotenCore Neural DSL — the human-readable scripting language |
| `.nod`    | `nod`     | KnotenCore JSON-AST — serialized Abstract Syntax Tree programs |

## Features (Phase 1)

- ✅ **Full syntax highlighting** for `.knoten` files:
  - All AST node keywords (`If`, `While`, `Assign`, `Lte`, `Gte`, `NotEq`, `And`, `Or`, `Not`, ...)
  - All `registry_*` FFI function calls (`registry_raycast_aabb`, `registry_get_mouse_ray`, ...)
  - Namespaced module calls: `registry.*`, `ui.*`, `fs.*`, `sys.*`, `time.*`, etc.
  - String literals, numeric literals (including `0xFF` hex), booleans
  - Comment lines (`# comment`)
  - Operators: `->`, `==`, `!=`, `<=`, `>=`, `&&`, `||`, `!`, bitwise ops
  - UI node names: `UIWindow`, `UILabel`, `UIButton`, `UITextInput`, `UIHBox`, `UIVBox`, ...
- ✅ **AST opcode highlighting** for `.nod` JSON-AST files
- ✅ **Bracket matching & auto-close** for `{}`, `[]`, `()`
- ✅ **Code snippets** for common patterns:
  - `kc-window` — WGPU window + game loop
  - `kc-raycast` — AABB raycast from mouse
  - `kc-uiwindow` — egui UIWindow with button
  - `kc-fn` — function definition
  - `kc-import` — import statement
  - `kc-while`, `kc-if` — control flow
  - `kc-aabb` — register world AABB
  - `kc-drawrect` — draw rect + label

## Installation (Local / Development)

1. Copy the `tools/vscode-knotencore/` directory to your VS Code extensions folder:
   - **Windows:** `%USERPROFILE%\.vscode\extensions\knotencore-0.1.0`
   - **macOS/Linux:** `~/.vscode/extensions/knotencore-0.1.0`
2. Restart VS Code.
3. Open any `.knoten` or `.nod` file — highlighting activates automatically.

### Install via VSIX (when packaged)

```bash
cd tools/vscode-knotencore
npm install -g @vscode/vsce
vsce package
code --install-extension vscode-knotencore-0.1.0.vsix
```

## Roadmap

- **Phase 2** — Language Server Protocol (LSP): diagnostics, hover docs, go-to-definition
- **Phase 3** — Marketplace publication & GitHub Linguist upstream submission

## Contributing

See [`CONTRIBUTING.md`](../../CONTRIBUTING.md) in the repository root.
