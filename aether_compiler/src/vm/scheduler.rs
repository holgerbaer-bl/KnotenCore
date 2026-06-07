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
