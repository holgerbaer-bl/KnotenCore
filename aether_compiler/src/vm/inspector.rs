// ── Sprint 303: inspector.rs — VM telemetry, hot-path profiling, ledger helpers ──
//
// Contains:
//   - VMInspectorData struct + VM::inspect()
//   - VM_INSPECTION_STATE (live IP/stack snapshot for egui overlay)
//   - HOT_PATH_TABLE (thread-local PGO profiling)
//   - Ledger helpers: compute_ledger_hash, verify_ledger_hash, get_ledger_nonce, LEDGER_NONCE
//   - VM_SLEEP_ACCUMULATED_MS (watchdog sleep-exclusion counter)
//
// Extracted from machine.rs (was lines 67–292) in Sprint 303.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

// ── Ledger ────────────────────────────────────────────────────────────────────

pub(super) static LEDGER_NONCE: AtomicU64 = AtomicU64::new(0);

/// Accumulated sleep duration (ms) injected by OpCode::Sleep — excluded from
/// the watchdog budget so long-running scripts with intentional sleeps don't
/// trigger a false timeout.
pub static VM_SLEEP_ACCUMULATED_MS: AtomicU64 = AtomicU64::new(0);

pub(super) fn compute_ledger_hash(crypto_hash: u64, nonce: u64, prev: &[u8; 32]) -> [u8; 32] {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    crypto_hash.hash(&mut h);
    nonce.hash(&mut h);
    prev.hash(&mut h);
    let digest = h.finish();
    let mut result = [0u8; 32];
    result[0..8].copy_from_slice(&digest.to_le_bytes());
    result[8..16].copy_from_slice(&digest.wrapping_mul(0x9E3779B9).to_le_bytes());
    result[16..24].copy_from_slice(&digest.wrapping_add(0x7F4A7C15).to_le_bytes());
    result[24..32].copy_from_slice(&prev[24..32]);
    result
}

/// Verifies that a `VMState`'s `previous_state_hash` is consistent with its own
/// `crypto_state_hash` and `nonce`. Used by snapshot resumption to detect tampering.
pub fn verify_ledger_hash(state: &super::machine::VMState) -> bool {
    let expected = compute_ledger_hash(state.crypto_state_hash, state.nonce, &[0u8; 32]);
    state.previous_state_hash == expected
}

/// Returns the current global ledger nonce (monotonically increasing).
pub fn get_ledger_nonce() -> u64 {
    LEDGER_NONCE.load(Ordering::SeqCst)
}

// ── Inspector Panel ───────────────────────────────────────────────────────────

/// Snapshot of VM metrics for the egui inspector panel (Sprint 297).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VMInspectorData {
    pub stack_depth: usize,
    pub frame_count: usize,
    pub ip: usize,
    pub bp: usize,
    pub crypto_state_hash: u64,
    pub ledger_nonce: u64,
    pub global_count: usize,
    pub active_isolates: usize,
    pub scheduler_harness_status: String,
}

/// Global live inspection state — written every instruction when `VM::is_inspectable`.
static VM_INSPECTION_STATE: std::sync::Mutex<Option<(usize, usize)>> = std::sync::Mutex::new(None);

pub(super) fn update_inspection_state(ip: usize, stack_depth: usize) {
    if let Ok(mut guard) = VM_INSPECTION_STATE.lock() {
        *guard = Some((ip, stack_depth));
    }
}

/// Returns the last recorded `(ip, stack_depth)` pair from an inspectable VM.
/// Returns `None` if no VM has run in inspectable mode yet.
pub fn get_vm_inspection_snapshot() -> Option<(usize, usize)> {
    if let Ok(guard) = VM_INSPECTION_STATE.lock() {
        *guard
    } else {
        None
    }
}

// ── Hot-Path Profiling (Sprint 256/282) ───────────────────────────────────────

thread_local! {
    /// Per-thread instruction-frequency table for adaptive PGO.
    /// Using thread-local storage prevents cross-test contamination
    /// and eliminates mutex overhead in the hot path.
    pub(super) static HOT_PATH_TABLE: std::cell::RefCell<HashMap<usize, usize>> =
        std::cell::RefCell::new(HashMap::new());
}

pub(super) fn track_hot_path(ip: usize) {
    HOT_PATH_TABLE.with(|t| {
        let mut map = t.borrow_mut();
        let count = map.entry(ip).or_insert(0);
        *count += 1;
        if *count == 10_000 {
            let marker = format!("HOT_PATH_BLOCK:IP:{ip}:HITS:10000");
            crate::natives::registry::registry_push_timing_marker(marker);
            eprintln!("[HotPath] IP {} reached 10k hits — marked as hot", ip);
        }
    });
}

#[allow(dead_code)]
pub(super) fn is_hot_path(ip: usize) -> bool {
    HOT_PATH_TABLE.with(|t| t.borrow().get(&ip).copied().unwrap_or(0) >= 10_000)
}

/// Drains the hot-path table for the current thread. Call between test runs
/// or when resetting PGO state.
pub fn drain_hot_path_table() {
    HOT_PATH_TABLE.with(|t| t.borrow_mut().clear());
}

// ── Crash Telemetry ───────────────────────────────────────────────────────────

pub(super) fn push_vm_crash_marker(ip: usize, stack_depth: usize, msg: &str) {
    let marker = format!("VM_CRASH_IP:{ip}:STACK:{stack_depth}:MSG:{msg}");
    crate::natives::registry::registry_push_timing_marker(marker);
}

// ── Opcode Hash (for crypto_state_hash rolling update) ───────────────────────

pub(super) fn opcode_discriminant_hash(op: &knoten_core_types::opcode::OpCode) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    std::mem::discriminant(op).hash(&mut h);
    h.finish()
}

// ── VMInspectorData construction helper ──────────────────────────────────────

/// Constructs a `VMInspectorData` snapshot from a running VM and the isolate registry.
pub fn build_inspector_data(
    stack_len: usize,
    frame_len: usize,
    ip: usize,
    bp: usize,
    crypto_state_hash: u64,
    globals_len: usize,
    active_isolates: usize,
) -> VMInspectorData {
    VMInspectorData {
        stack_depth: stack_len,
        frame_count: frame_len,
        ip,
        bp,
        crypto_state_hash,
        ledger_nonce: get_ledger_nonce(),
        global_count: globals_len,
        active_isolates,
        scheduler_harness_status: "n/a (local test harness)".to_string(),
    }
}
