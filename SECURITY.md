# Security Policy

## Supported Versions

Currently, KnotenCore is in active Alpha development. We provide security support for the latest branch.

| Version | Supported          |
| ------- | ------------------ |
| v1.0.x  | :white_check_mark: |
| < v1.0  | :x:                |

## Architecture Context

KnotenCore is engineered as a **Sandboxed Engine**. Security forms the bedrock of our architecture:
- All disk and I/O interactions are strictly gated through `executor::ExecutionEngine::validate_fs_path`, actively rejecting unauthorized path traversals.
- The Engine explicitly sanitizes Windows UNC prefixes (`\\?\`) via `dunce::canonicalize`.
- Foreign Function Interface (FFI) bindings enforce explicit whitelist permission checks. No arbitrary system binaries can be invoked unknowingly.

Because of this strict sandbox environment, **sandbox escapes, arbitrary code executions, or permission bypasses are considered critical security vulnerabilities** and receive our highest priority.

## Reporting a Vulnerability

**Do NOT report security vulnerabilities in public GitHub issues.**

If you discover a vulnerability, please report it privately:
1. Open a **GitHub Security Advisory** directly on this repository.
2. Provide a detailed explanation of the exploit, including minimal reproduction steps (a `.nod` or `.knoten` script that triggers the issue).
3. If applicable, specify the Operating System and Native Rust Architecture where the bypass is viable.

We will acknowledge your report within 48 hours and work with you on a patch before publicly disclosing the issue. 
