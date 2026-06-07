use super::machine::VMState;
use std::collections::HashMap;

// Sprint 263: Atomic isolate snapshot synchronization
static ISOLATE_SNAPSHOTS: std::sync::OnceLock<std::sync::Mutex<HashMap<i64, VMState>>> =
    std::sync::OnceLock::new();

fn get_snapshot_registry() -> &'static std::sync::Mutex<HashMap<i64, VMState>> {
    ISOLATE_SNAPSHOTS.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

pub fn snapshot_isolate(isolate_id: i64) -> Option<VMState> {
    let registry = get_snapshot_registry();
    let guard = registry.lock().unwrap_or_else(|e| e.into_inner());
    guard.get(&isolate_id).cloned()
}

pub fn store_snapshot(isolate_id: i64, state: VMState) {
    let registry = get_snapshot_registry();
    let mut guard = registry.lock().unwrap_or_else(|e| e.into_inner());
    guard.insert(isolate_id, state);
}

pub fn rollback_isolate(isolate_id: i64, state: VMState) -> bool {
    let registry = get_snapshot_registry();
    let mut guard = registry.lock().unwrap_or_else(|e| e.into_inner());
    guard.insert(isolate_id, state);
    true
}

pub fn drain_isolate_snapshots() {
    if let Some(reg) = ISOLATE_SNAPSHOTS.get() {
        reg.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
}
