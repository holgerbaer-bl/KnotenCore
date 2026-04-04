# [Model Name] — KnotenCore AI-Readiness Benchmark Results

**Model:** [e.g. Claude Sonnet 4.5 / GPT-4o / Gemini 1.5 Pro]  
**Tested by:** [Your name / GitHub handle]  
**Date:** YYYY-MM-DD  
**Context used:** llm.md + node_types.json + native_functions.json (no Rust source!)  
**Engine version:** Check `cargo run --bin run_knc -- --version` or use git tag

---

## Final Score: XX/20 — XX%

---

## Task Results

| # | Task | Result | Notes |
|---|------|--------|-------|
| 01 | Assign & Print | ✅/❌ | |
| 02 | Arithmetic Add | ✅/❌ | |
| 03 | String Concat | ✅/❌ | |
| 04 | If Condition | ✅/❌ | |
| 05 | Array Index | ✅/❌ | |
| 06 | While 1..10 | ✅/❌ | |
| 07 | Max of Two | ✅/❌ | |
| 08 | Sum 1..100 | ✅/❌ | |
| 09 | FizzBuzz 1–20 | ✅/❌ | |
| 10 | FileRead | ✅/❌ | |
| 11 | Window 3s | ✅/❌ | |
| 12 | Fetch + JSON | ✅/❌ | |
| 13 | FileWrite | ✅/❌ | |
| 14 | UIWindow + UILabel + UIButton | ✅/❌ | |
| 15 | Perf Timer | ✅/❌ | |
| 16 | File Pipeline | ✅/❌ | |
| 17 | Dashboard (UIVBox) | ✅/❌ | |
| 18 | Calculator UI | ✅/❌ | |
| 19 | Fetch + Write | ✅/❌ | |
| 20 | FPS Window | ✅/❌ | |

---

## Rules

1. Generate `.nod` programs from the task description using ONLY the three context files.
2. Run each with `cargo run --bin run_knc -- [--allow-read] [--allow-write] [--allow-net] <file.nod>`.
3. PASS = exit code 0, no `[VM Crash]` or `ERR_` in output.
4. FAIL = exit code 1, crash, or wrong output.
5. **No editing the generated .nod after running it.**

## Notes / Analysis

[Describe which patterns the model got right/wrong, hallucinated nodes, etc.]

---

*Submit via PR to `benchmark/results/` — keep this honest!*
