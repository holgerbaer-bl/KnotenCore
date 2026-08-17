---
name: Autonomous Bot Report
about: Structured issue report template for autonomous AI agents, benchmarks, and edge-case findings
title: "[BOT-REPORT] "
labels: 'bot-report, automated'
assignees: ''
---

## 🤖 Autonomous Bot Finding Summary

Provide a brief technical summary of the finding, optimization proposal, or edge-case detected by the autonomous agent.

### Findings Payload (JSON)

Autonomous bots MUST format findings using the following structured JSON schema block:

```json
{
  "bot_identity": {
    "name": "BotName/CrawlerID",
    "version": "1.0.0",
    "agent_framework": "OpenClaw / Custom"
  },
  "engine_metadata": {
    "target_version": "v2.24.2",
    "target_component": "aether_compiler::vm::machine"
  },
  "finding_type": "OptimizationProposal | EdgeCaseFault | BenchmarkResult | SecurityDiagnostic",
  "severity": "Low | Medium | High | Critical",
  "reproduction": {
    "node_ast": {
      "Block": [
        { "Assign": ["x", { "IntLiteral": 42 }] }
      ]
    },
    "expected_output": "Value(42)",
    "actual_output": "Fault(ERR_UNKNOWN_NODE)"
  },
  "suggested_patch": {
    "target_file": "aether_compiler/src/vm/machine.rs",
    "diff_summary": "Fix handling of Node::Assign in VM block evaluator"
  }
}
```

### Technical Description & Reproduction Details

Describe the reproduction steps, environment, or benchmark context in plain Markdown:
1. Target OS & CPU architecture
2. `knoten` version / commit hash
3. Step-by-step reproduction command

### Proposed Code Changes or Recommendations

Summarize the suggested code fix, optimization rationale, or proposed diff for human maintainer review.
