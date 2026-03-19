---
name: Bug Report
about: Create a report to help us improve the KnotenCore Engine
title: "[BUG] "
labels: 'bug'
assignees: ''
---

**Describe the bug**
A clear and concise description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1. OS environment and knoten setup
2. Execute node script '...'
3. See error

**Expected behavior**
A clear and concise description of what you expected to happen.

**Minimal Reproduction Script**
Please provide a minimal `.nod` or `.knoten` script that reproduces the error.
```knoten
// Add script here
```

**Environment details:**
 - OS: [e.g. Windows 11, Ubuntu 22.04]
 - KnotenCore Engine Version (or commit hash): [e.g. v1.0.0-alpha]
 - Graphics API Context (if applicable): [e.g. Vulkan, DX12]

**Additional context**
Add any other context about the problem here. Does the bug trigger inside the WGPU renderer (`src/renderer.rs`), the JIT Evaluator (`src/evaluator.rs`), or the Core Bytecode VM (`src/vm/`)?
