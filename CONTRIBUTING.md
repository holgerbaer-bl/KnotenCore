# Contributing to KnotenCore

Welcome, and thank you for your interest in contributing to KnotenCore! 

This repository is maintained by the architect and often evolved by autonomous agents. To ensure the integrity of the project, please review these architectural foundations taking precedence over all PRs.

## Our Architectural Pillars

KnotenCore is a high-performance hybrid language (JIT/AOT) offering native WGPU hardware rendering and deterministic memory management.

Before contributing, you **must** understand the Execution Backend:
1. **The Bytecode VM (`src/vm/`)**: Following the v1.0.0-alpha architecture, KnotenCore is migrating towards an Ahead-of-Time (AOT) Virtual Machine. 
2. **The Transpiler (`compiler.rs`)**: We compile AST nodes using **Reverse Polish Notation (RPN)** logic (evaluating Left, evaluating Right, then evaluating Operator). Control flow evaluates via `Compiler Backpatching`.
3. **The Dispatcher (`machine.rs`)**: The engine runs as a blisteringly fast **Stack Machine**. It maintains zero heap allocations computationally, operating via an Arithmetic Logic Unit (ALU) that explicitly pops values off the stack, calculates native Rust mathematics, and repushes.
4. **The Sandbox (`executor.rs`)**: File system access and OS operations are stringently whitelisted. Never bypass `validate_fs_path`.

## Pull Request Process

1. **Ensure Tests Pass**: Run `cargo test` prior to opening your PR. With over 50 native Knotencore scripts in test coverage, breaking the JIT evaluator or the AOT Bytecode machine will cause the CI to fail immediately.
2. **Review the PR Template**: Our repository uses a `.github/PULL_REQUEST_TEMPLATE.md`. You must fill out the checklist confirming whether or not your code modifies the FFI permissions or the VM Core (`src/vm/`).
3. **Draft Meaningful Commits**: The repository follows strict, descriptive sprint commit messages natively managed by AI agents.
4. **Code Format**: Always run `cargo fmt` and `cargo clippy`. We enforce zero-latency cross-thread patterns (Winit EventLoopProxies) over slow channels.

Thank you for expanding the engine!
