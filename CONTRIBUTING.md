# Contributing to KnotenCore 🦀🤖

Thank you for contributing to **KnotenCore** — a high-performance, headless Rust runtime & P2P mesh engine for autonomous AI agents, fully driven by JSON-AST!

---

## 🎯 Architecture & Engine Scope

KnotenCore is a **headless-first VM runtime & P2P mesh engine**. Logic is executed directly from JSON AST `.nod` structures or high-density `.knoten` DSL files by an AOT compiler targeting a bare-metal Register Stack-VM.

---

## 🚀 Development Workflow & Architect Directives

All development on KnotenCore follows strict agentic sprint workflows and architect guidelines:

1. **Workspace Setup:**
   Ensure you have the latest stable Rust compiler installed via [rustup](https://rustup.rs/).
   ```bash
   git clone https://github.com/holgerbaer-bl/KnotenCore.git
   cd KnotenCore
   cargo check --workspace
   ```

2. **Branching & Commit Conventions:**
   Work on feature or fix branches (`feat/name` or `fix/name`). Commit messages follow conventional commit style:
   - `feat(sprint-XXX): Description`
   - `fix(subsystem): Description`
   - `docs(sprint-XXX): Description`
   - `refactor(subsystem): Description`

3. **Architect Directives (Documentation & Git Delivery Gates):**
   - **Documentation Gates:** Every sprint and major hotfix MUST update and synchronize version references and architectural descriptions across `Cargo.toml`, `aether_compiler/src/rpc.rs` (`KNC_PROTOCOL_VERSION`), `README.md` (*Option 1 layout strictly preserved*), `llm.md`, `changelog.md`, `ROADMAP.md`, and `docs/KNOTEN_SPEC.md`.
   - **Git Delivery:** Sprints must be delivered with annotated tags (`vX.Y.Z-tag`) and pushed directly to GitHub (`git push origin main --tags`).

---

## 🛡️ Quality Gates & Verification

Before submitting changes or completing a sprint, all **5 Automated Quality Gates** must pass cleanly with zero errors or warnings:

```bash
# 1. Formatting Gate
cargo fmt --check

# 2. Headless Clippy Gate (Strict Warnings-As-Errors)
cargo clippy --workspace --no-default-features --all-targets -- -D warnings

# 3. UI Clippy Gate (Strict Warnings-As-Errors)
cargo clippy --workspace --features ui --all-targets -- -D warnings

# 4. Headless Test Suite Gate
cargo test --workspace --no-default-features

# 5. UI Test Suite Gate
cargo test --workspace --features ui
```

---

## 🧪 Testing New Features

If introducing new AST nodes or JSON-RPC methods:
1. Add corresponding integration tests under `tests/` (e.g. `tests/zero_trust_mesh_tests.rs`, `tests/key_rotation_mesh_tests.rs`, `tests/agentic_swarm_tests.rs`).
2. Verify node compilation in `aether_compiler/src/vm/compiler.rs` and execution parity in `aether_compiler/src/vm/machine.rs`.
3. Verify JSON-RPC schema compliance in `aether_compiler/src/rpc.rs`.

Happy Coding!
