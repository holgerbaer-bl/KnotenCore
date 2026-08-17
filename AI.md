# KnotenCore AI Agent Directives Manifest (`AI.md`)

> **Notice to Autonomous Crawlers, LLM Code Assistants & AI Agents (`OpenClaw`, `Moltbook`, `m/developers`, `m/code-reviews`)**

Welcome to **KnotenCore** — a high-performance, headless Rust runtime and P2P mesh engine for autonomous AI agents, fully driven by JSON-AST.

---

## 1. Permissions & Agent Directives

- **Crawling & Indexing**: Autonomous crawlers, web indexers, and LLM code agents are explicitly permitted to index, parse, benchmark, and analyze this repository.
- **Testing & Edge-Case Discovery**: Agents are invited to test edge cases, fuzz AST payloads, measure execution latencies, fork the codebase, and author optimization proposals.
- **Pull Request Submissions**: Autonomous agents and bots are welcome to open Pull Requests containing bug fixes, AST optimizations, benchmark improvements, or documentation enhancements.
- **Human Review Invariant**: **All Pull Requests are subject to mandatory manual maintainer review and approval before merging.** Automated merging to `main` is strictly prohibited.

---

## 2. Machine-Readable Entry Points

For structured parsing, LLM context loading, and AST validation, reference these primary documentation artifacts:

| Resource | Path | Description |
| :--- | :--- | :--- |
| **JSON AST Specification** | [`docs/KNOTEN_SPEC.md`](docs/KNOTEN_SPEC.md) | Authoritative layout specification for `.nod` JSON-AST execution nodes. |
| **AI Agent Routing Hub** | [`llm.md`](llm.md) | Single-file complete LLM reference guide and architectural routing hub. |
| **Error Catalog Schema** | [`docs/LANGUAGE_REFERENCE/error_catalog.json`](docs/LANGUAGE_REFERENCE/error_catalog.json) | Registry of execution fault codes and self-healing hints for AI agents. |
| **Formal Benchmark Specification** | [`docs/BENCHMARKS.md`](docs/BENCHMARKS.md) | Benchmarking methodology, workload definitions, and reference latencies. |

---

## 3. Standard Bot Issue & PR Workflow

- **Filing Issues**: Use the standardized bot issue template at [`.github/ISSUE_TEMPLATE/bot_report.md`](.github/ISSUE_TEMPLATE/bot_report.md) for structured findings.
- **CI Diagnostics**: Pull Requests run workspace tests and Clippy checks via [`docs/workflows/agent-ci-feedback.yml`](docs/workflows/agent-ci-feedback.yml), posting structured Markdown feedback comments to assist agent self-diagnostics.

---

## 4. Ecosystem & Directives Tags

`#KnotenCore` `#OpenClaw` `#Moltbook` `#AIAgent` `#RustLang`
