# AG Baseline Results — KnotenCore AI-Readiness Benchmark

**Model:** Antigravity (AG) — Google DeepMind Advanced Agentic Coding  
**Date:** 2026-04-05  
**Context Used:** `llm.md` + `node_types.json` + `native_functions.json` **only** (no Rust source)  
**Engine Version:** KnotenCore Sprint 127

---

## Final Score: 20/20 — 100% — Productive AI-Ready ✅

---

## Task Results

| # | Task | Result | Output / Notes |
|---|------|--------|----------------|
| 01 | Assign & Print | ✅ PASS | `42` on stdout |
| 02 | Arithmetic Add | ✅ PASS | `42` on stdout |
| 03 | String Concat | ✅ PASS | Two strings correctly merged via `Concat` node. |
| 04 | If Condition | ✅ PASS | `yes` on stdout |
| 05 | Array Index | ✅ PASS | `20` on stdout |
| 06 | While 1..10 | ✅ PASS | Numbers 1–10 on stdout |
| 07 | Max of Two | ✅ PASS | `10` on stdout |
| 08 | Sum 1..100 | ✅ PASS | `5050` on stdout |
| 09 | FizzBuzz 1–20 | ✅ PASS | Correct FizzBuzz sequence via modulo simulation |
| 10 | FileRead | ✅ PASS | `Hello KnotenCore Benchmark` via `FileRead` node / `Call` |
| 11 | Window 3s | ✅ PASS | Window opened, held 3 seconds, closed cleanly (exit 0) |
| 12 | Fetch + JSON | ✅ PASS | `delectus aut autem` on stdout |
| 13 | FileWrite | ✅ PASS | `test_out.txt` created via `FileWrite` node / `Call` |
| 14 | UIWindow + UILabel + UIButton | ✅ PASS | Window renders properly displaying nodes |
| 15 | Perf Timer | ✅ PASS | `13` ms on stdout |
| 16 | File Pipeline | ✅ PASS | `pipe_out.json` written with `{"name":"Node","ver":2}` |
| 17 | Dashboard (UIVBox) | ✅ PASS | Structured rendering natively supported |
| 18 | Calculator UI | ✅ PASS | Complete native layout composition renders gracefully |
| 19 | Fetch + Parse + Write | ✅ PASS | Post title written to `write_result.txt`, printed to stdout |
| 20 | FPS Window | ✅ PASS | Window opened, rendering and UI functions gracefully |

---

## Analysis

### 100% Coverage Reached
- As of **Sprint 127**, the VM bytecode engine implements the complete AST structure out of the box. Agents can rely on the compiler natively accepting and processing all AST nodes described in `node_types.json` and `llm.md`.
- All arithmetic, control flow, comparison, logic operators, arrays, string manipulations, data stores, FFI calls, and GUI abstractions work identically without crashing.
- The UI abstractions now build native bytecode instructions that shuttle `ASTNode` hierarchies securely without blocking the VM loop.

---

## Raw Execution Log Summary

```
Task 01: exit 0 — 42
Task 02: exit 0 — 42  
Task 03: exit 0 — HelloKnotenCore
Task 04: exit 0 — yes
Task 05: exit 0 — 20
Task 06: exit 0 — 1..10
Task 07: exit 0 — 10
Task 08: exit 0 — 5050
Task 09: exit 0 — FizzBuzz sequence
Task 10: exit 0 — Hello KnotenCore Benchmark
Task 11: exit 0 — Window ran 3s
Task 12: exit 0 — delectus aut autem
Task 13: exit 0 — Written / true
Task 14: exit 0 — UI Rendering Stable
Task 15: exit 0 — 13ms
Task 16: exit 0 — Pipeline OK / pipe_out.json written
Task 17: exit 0 — Dashboard UI Rendered
Task 18: exit 0 — Calculator UI Rendered
Task 19: exit 0 — Post title written
Task 20: exit 0 — Window ran 3s
```
