# Contributing to KnotenCore

First off, thank you for considering contributing to KnotenCore! It's people like you that make KnotenCore an enterprise-grade Virtual Machine architecture.

## Getting Started

1. **Fork & Clone:** Fork the repository on GitHub and clone your fork locally.
   ```bash
   git clone https://github.com/YOUR_USERNAME/KnotenCore.git
   cd KnotenCore
   ```
2. **Setup Rust:** Ensure you have the latest stable Rust compiler installed via [rustup](https://rustup.rs/).
3. **Build the Engine:** Run the following command from the `aether_compiler` root to verify your workspace is intact.
   ```bash
   cargo build
   ```

## Finding an Issue
We curate issues labeled `good first issue` to help you onboard without needing deep knowledge of the AOT Compiler or Virtual Machine internals. These issues explicitly target the `core/` Standard Library. 

## Testing Your Changes
The KnotenCore engine relies heavily on its integration safety sandbox.
Before submitting your Pull Request, you **must** ensure the compiler unit tests and deterministic sandbox validation checks pass successfully:
```bash
cargo clippy --lib
cargo test --lib
```

If your changes involve new `.nod` script functionality:
```bash
cargo run --bin run_knc -- examples/your_test_script.nod
```

## Submitting a Pull Request
1. Create a branch (`git checkout -b feature/your-feature-name`).
2. Make your logical, isolated commits.
3. Push your branch to your fork.
4. Open a Pull Request targeting our `main` branch. Provide a clear summary of your changes.

Happy Coding!
