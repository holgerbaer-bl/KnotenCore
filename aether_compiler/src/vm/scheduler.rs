use crate::executor::RelType;
use knoten_core_types::opcode::OpCode;
use std::collections::HashMap;

// Sprint 262: Deterministic work-stealing scheduler
pub type WorkItem = (OpCode, Vec<RelType>);
pub type WorkQueue = std::collections::VecDeque<WorkItem>;
pub type WorkQueueMap = HashMap<i64, WorkQueue>;

static WORK_STEALING_QUEUES: std::sync::OnceLock<std::sync::Mutex<WorkQueueMap>> =
    std::sync::OnceLock::new();

fn get_work_queues() -> &'static std::sync::Mutex<WorkQueueMap> {
    WORK_STEALING_QUEUES.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

pub fn push_work_batch(isolate_id: i64, work: Vec<WorkItem>) {
    let queues = get_work_queues();
    let mut guard = queues.lock().unwrap_or_else(|e| e.into_inner());
    guard.entry(isolate_id).or_default().extend(work);
}

pub fn try_steal_work(thief_id: i64) -> Option<(OpCode, Vec<RelType>)> {
    let queues = get_work_queues();
    let mut guard = queues.lock().unwrap_or_else(|e| e.into_inner());
    for (&victim_id, victim_queue) in guard.iter_mut() {
        if victim_id != thief_id && !victim_queue.is_empty() {
            return victim_queue.pop_front();
        }
    }
    None
}

pub fn drain_work_stealing_queues() {
    if let Some(queues) = WORK_STEALING_QUEUES.get() {
        queues.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
}

// Sprint 274: Speculative branch dispatch — runs both true/false paths in parallel shadow isolates
use super::machine::VMState;

pub fn dispatch_speculative_branch(
    snapshot: VMState,
    path_a_instructions: Vec<OpCode>,
    path_b_instructions: Vec<OpCode>,
    constants: Vec<RelType>,
    take_path_a: bool,
) -> Option<VMState> {
    let handle_a = super::isolate::spawn_shadow_isolate(
        snapshot.clone(),
        path_a_instructions,
        constants.clone(),
    );
    let handle_b = super::isolate::spawn_shadow_isolate(snapshot, path_b_instructions, constants);

    let result_a = handle_a.join().ok()?;
    let result_b = handle_b.join().ok()?;

    if take_path_a {
        let _ = result_b;
        Some(result_a.state)
    } else {
        let _ = result_a;
        Some(result_b.state)
    }
}
