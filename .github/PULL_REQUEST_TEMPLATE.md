## Description
Please include a summary of the change and which issue is fixed. Please also include relevant motivation and context.

Fixes # (issue)

## Type of change
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update

## Architectural Checklist:
Please review the contribution guidelines in `CONTRIBUTING.md` before submitting.

- [ ] My code follows the code conventions of this project (`cargo clippy`, `cargo fmt`).
- [ ] I have executed the testing suite (`cargo test`) and no integration tests or VM tests are broken.
- [ ] I have written my own unit/integration tests verifying the new logic.
- [ ] **FFI/VM Check**: Does this PR modify `executor.rs` paths, WGPU bindings, or the AOT `src/vm/` Bytecode Machine? (If yes, please highlight the modifications explicitly for review).

## Testing context
Describe the tests that you ran to verify your changes. Include any synthetic scripts or visual context needed for review.
