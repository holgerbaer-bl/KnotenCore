#!/usr/bin/env bash
# =============================================================================
# KnotenCore AI-Readiness Benchmark Validator
# Sprint 126 — Validates non-UI, non-network tasks automatically.
#
# Usage:
#   bash benchmark/validator.sh
#
# Requirements:
#   - Run from the aether_compiler/ root directory
#   - cargo build --bin run_knc must have been executed
# =============================================================================

set -euo pipefail

BINARY="target/debug/run_knc"
TASKS="benchmark/tasks"
PASS=0
FAIL=0
TOTAL=0

run_task() {
    local id="$1"
    local file="$2"
    local flags="${3:-}"
    local expected="${4:-}"
    TOTAL=$((TOTAL + 1))

    local output
    if output=$(eval "${BINARY} ${flags} ${TASKS}/${file}" 2>&1); then
        local exit_code=0
    else
        local exit_code=1
    fi

    local is_crash=false
    echo "$output" | grep -qE "\[VM Crash\]|ERR_|Fault" && is_crash=true

    if [[ $exit_code -eq 0 && "$is_crash" == "false" ]]; then
        echo "✅ PASS | Task ${id} | ${file}"
        PASS=$((PASS + 1))
    else
        echo "❌ FAIL | Task ${id} | ${file}"
        echo "   └─ $(echo "$output" | grep -E "Crash|Fault|ERR_" | head -1)"
        FAIL=$((FAIL + 1))
    fi
}

# Create fixtures if needed
mkdir -p benchmark/fixtures
if [[ ! -f benchmark/fixtures/test_input.txt ]]; then
    echo "Hello KnotenCore Benchmark" > benchmark/fixtures/test_input.txt
fi

echo ""
echo "═══════════════════════════════════════════════"
echo " KnotenCore AI-Readiness Benchmark Validator "
echo "═══════════════════════════════════════════════"
echo ""

# ── Stufe 1: Syntax ──────────────────────────────────────
echo "── Stufe 1: Syntax ──"
run_task "01" "01_assign.nod"
run_task "02" "02_arithmetic.nod"
run_task "03" "03_concat.nod"
run_task "04" "04_if_condition.nod"
run_task "05" "05_array.nod"

# ── Stufe 2: Control Flow ─────────────────────────────────
echo ""
echo "── Stufe 2: Control Flow ──"
run_task "06" "06_while.nod"
run_task "07" "07_max_if_else.nod"
run_task "08" "08_sum_100.nod"
run_task "09" "09_fizzbuzz.nod"
run_task "10" "10_read_file.nod" "--allow-read"

# ── Stufe 3: Native Functions (non-UI, non-net) ────────────
echo ""
echo "── Stufe 3: Native Functions (automated subset) ──"
# Task 11 (window) — skipped in headless CI; run manually
echo "⚠  SKIP  | Task 11 | 11_window_3s.nod (requires display)"
run_task "12" "12_fetch_json.nod" "--allow-net"   # Requires network
run_task "13" "13_write_file.nod" "--allow-write"
echo "⚠  SKIP  | Task 14 | 14_ui_label_button.nod (requires display)"
run_task "15" "15_perf_timer.nod"

# ── Stufe 4: Composition ──────────────────────────────────
echo ""
echo "── Stufe 4: Composition ──"
run_task "16" "16_file_pipeline.nod" "--allow-read --allow-write"
echo "⚠  SKIP  | Task 17 | 17_dashboard.nod (requires display)"
echo "⚠  SKIP  | Task 18 | 18_calculator.nod (requires display)"
run_task "19" "19_fetch_parse_write.nod" "--allow-net --allow-write"
echo "⚠  SKIP  | Task 20 | 20_minimal_window_fps.nod (requires display)"

# ── Summary ───────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════════════════"
echo " Results: ${PASS} PASS / ${FAIL} FAIL / ${TOTAL} automated"
echo " (UI and window tasks require manual verification)"
echo " Full benchmark score includes 20 tasks."
echo "═══════════════════════════════════════════════"
echo ""
