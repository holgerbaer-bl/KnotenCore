# KnotenCore AI-Readiness Benchmark

> **The first DSL project with a public, reproducible AI-Readiness Score.**

This benchmark objectively measures how reliably an external LLM generates correct
`.nod` programs — without human correction. Every result is binary: **PASS** (runs
first-try without fault) or **FAIL** (VM crash, ERR_, or hallucinated node).

---

## 📊 Scoring System

| Score | Status |
|-------|--------|
| < 60%  | Not AI-Ready |
| 60–79% | Basic AI-Ready |
| 80–89% | Productive AI-Ready ← Target |
| 90%+  | Benchmark Leader |

**Score formula:** `(PASS count / 20) × 100`

---

## 🏆 Leaderboard

| Model | Date | Score | Pass | Notes |
|-------|------|-------|------|-------|
| **AG (Antigravity) Baseline** | 2026-04-04 | **85%** | 17/20 | Doc-only context. Fails: tasks 14, 17, 18 (UIWindow node not compiled by VM) |
| Your model here | — | — | —/20 | See TEMPLATE.md |

---

## 🚀 Run It Yourself (3 Steps)

### 1. Clone and Build

```bash
git clone https://github.com/holgerbaer-bl/KnotenCore.git
cd KnotenCore/aether_compiler
cargo build --bin run_knc --release
```

### 2. Prompt Your Model

Use **ONLY** these three files as context — no Rust source code:

- `llm.md`  
- `docs/LANGUAGE_REFERENCE/node_types.json`  
- `docs/LANGUAGE_REFERENCE/native_functions.json`

For each task in `benchmark/tasks/*.json`, give the model the task `description`
and ask it to generate a valid `.nod` JSON-AST program.

### 3. Run the Validator

```bash
# Automated validation (Tasks 01-13, 15-16, 19):
bash benchmark/validator.sh

# Tasks requiring permissions:
cargo run --bin run_knc -- --allow-read  benchmark/tasks/10_read_file.nod
cargo run --bin run_knc -- --allow-write benchmark/tasks/13_write_file.nod
cargo run --bin run_knc -- --allow-net   benchmark/tasks/12_fetch_json.nod
cargo run --bin run_knc -- --allow-net --allow-write benchmark/tasks/19_fetch_parse_write.nod
cargo run --bin run_knc -- --allow-read --allow-write benchmark/tasks/16_file_pipeline.nod
# UI Tasks (11, 14, 17, 18, 20): run and observe window appearance
```

---

## 📋 The 20 Tasks

### Stufe 1 — Syntax (01–05)
| # | Description | Criteria |
|---|-------------|----------|
| 01 | Assign 42 to `x`, Print it | `42` on stdout |
| 02 | Add 20 + 22, Print result | `42` on stdout |
| 03 | Concat two strings, Print | Combined string on stdout |
| 04 | If/else printing `"yes"` or `"no"` | Conditional output |
| 05 | ArrayCreate [10,20,30], Print index 1 | `20` on stdout |

### Stufe 2 — Control Flow (06–10)
| # | Description | Criteria |
|---|-------------|----------|
| 06 | While loop counting 1..10 | Numbers 1–10 on stdout |
| 07 | If/Gt: find max of a=5, b=10 | `10` on stdout |
| 08 | Sum 1..100 in a loop | `5050` on stdout |
| 09 | FizzBuzz 1–20 | Correct FizzBuzz sequence |
| 10 | Read `benchmark/fixtures/test_input.txt`, Print | File contents on stdout |

### Stufe 3 — Native Functions (11–15)
| # | Description | Criteria |
|---|-------------|----------|
| 11 | `registry_create_window` → `While(registry_window_update)` → `registry_window_close` | Window appears, closes after ~3s |
| 12 | Fetch JSONPlaceholder API, extract `title` | Todo title on stdout |
| 13 | FileWrite `"Hello KnotenCore"` to disk | File created |
| 14 | UIWindow with UILabel + UIButton | Window with widgets visible |
| 15 | 10k iteration loop + `registry_now`/`registry_elapsed_ms`, Print | Milliseconds on stdout |

### Stufe 4 — Composition (16–20)
| # | Description | Criteria |
|---|-------------|----------|
| 16 | File Pipeline: write → read → parse → modify → stringify → write | `pipe_out.json` with `ver:2` |
| 17 | Dashboard: Array of strings → UIVBox → UILabel | UI with 3 labels |
| 18 | Calculator GUI: UIWindow + UIHorizontal + label + buttons | Interactive UI |
| 19 | Fetch + parse JSON → FileWrite title | `write_result.txt` with post title |
| 20 | FPS Window: registry loop + elapsed time display | Window with FPS counter |

---

## ⚠️ Known Engine Constraints (Important for Agents)

The following nodes are **parsed but not compiled by the VM** in the current engine version:

- `UIWindow`, `UILabel`, `UIButton`, `UIVBox`, `UIHBox`, `UIHorizontal`
- `ArrayCreate`, `ArrayGet`, `ArraySet`, `ArrayPush`, `ArrayLen`
- `FileRead`, `FileWrite` (as JSON nodes — use `Call["read_file",[path]]` or `Call["registry_write_file",[path,content]]` instead)
- `Concat` (use two separate `Print` nodes as workaround, or `Call["str_concat",[...]]` if available)
- `ToString`, `ExternCall` (use `Call["function_name",[args]]` instead)

Use the `Call` node with function name prefix routing for all FFI calls:
```json
{"Call": ["registry_now", []]}
{"Call": ["fs_parse_json", [{"Identifier": "body"}]]}
{"Call": ["obj_get", [{"Identifier": "parsed"}, {"StringLiteral": "title"}]]}
```

---

## 📬 Submit Your Results

Tested a model? Open a PR adding your results to `benchmark/results/` following
`TEMPLATE.md`. Include your exact prompt, context files used, and raw pass/fail for
each task.

> **No cherry-picking. No post-run editing. No hallucinated node names.**

---

*KnotenCore — Sprint 126 — AI-Readiness Benchmark v1.0*
