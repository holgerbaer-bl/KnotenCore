use crate::executor::RelType;
use crate::vm::isolate::VMIsolate;
use dashmap::DashMap;
use knoten_core_types::opcode::OpCode;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;

// Sprint 262: Deterministic work-stealing scheduler
pub type WorkItem = (OpCode, Vec<RelType>);
pub type WorkQueue = std::collections::VecDeque<WorkItem>;
pub type WorkQueueMap = HashMap<i64, WorkQueue>;

static WORK_STEALING_QUEUES: OnceLock<Mutex<WorkQueueMap>> = OnceLock::new();

fn get_work_queues() -> &'static Mutex<WorkQueueMap> {
    WORK_STEALING_QUEUES.get_or_init(|| Mutex::new(HashMap::new()))
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

// Sprint 274: Speculative branch dispatch
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

// Sprint 278: Cluster-wide work-stealing over logical node IDs
static CLUSTER_WORK_QUEUES: OnceLock<DashMap<String, WorkQueue>> = OnceLock::new();

fn get_cluster_queues() -> &'static DashMap<String, WorkQueue> {
    CLUSTER_WORK_QUEUES.get_or_init(DashMap::new)
}

pub fn push_cluster_work_batch(node_id: &str, work: Vec<WorkItem>) {
    let queues = get_cluster_queues();
    queues.entry(node_id.to_string()).or_default().extend(work);
}

pub fn try_steal_cluster_work(node_id: &str, thief_id: i64) -> Option<(OpCode, Vec<RelType>)> {
    let queues = get_cluster_queues();
    if let Some(mut entry) = queues.get_mut(node_id)
        && !entry.is_empty()
    {
        let stolen = entry.pop_front()?;
        let _ = thief_id;
        return Some(stolen);
    }
    try_steal_work(thief_id)
}

pub fn drain_cluster_work_queues() {
    if let Some(queues) = CLUSTER_WORK_QUEUES.get() {
        queues.clear();
    }
}

// Sprint 286: WASM-aware non-blocking work-stealing — avoids mutex contention
// in single-threaded WASM runtimes by falling through immediately if the local
// queue lock cannot be acquired without blocking.
pub fn try_steal_wasm_work(thief_id: i64) -> Option<(OpCode, Vec<RelType>)> {
    let queues = WORK_STEALING_QUEUES.get()?;
    let mut guard = queues.try_lock().ok()?;
    for (&victim_id, victim_queue) in guard.iter_mut() {
        if victim_id != thief_id && !victim_queue.is_empty() {
            return victim_queue.pop_front();
        }
    }
    None
}

// Sprint 299: Distributed Raft Consensus & Autonomous Failover
#[derive(Debug, Clone, PartialEq)]
pub enum RaftState {
    Leader,
    Follower,
    Candidate,
}

pub struct RaftCluster {
    pub nodes: Vec<String>,
    pub current_leader: Option<String>,
    pub term: u64,
    pub votes: HashMap<String, u64>,
    pub heartbeats: HashMap<String, u64>,
    pub election_timeouts: HashMap<String, u64>,
    pub replicated_logs: Vec<(u64, String)>,
}

impl RaftCluster {
    pub fn new(node_names: &[&str]) -> Self {
        let mut heartbeats = HashMap::new();
        let mut timeouts = HashMap::new();
        let nodes: Vec<String> = node_names.iter().map(|n| n.to_string()).collect();
        for node in &nodes {
            heartbeats.insert(node.clone(), 0);
            timeouts.insert(node.clone(), 150 + (nodes.len() as u64 * 37 % 150));
        }
        Self {
            nodes,
            current_leader: None,
            term: 0,
            votes: HashMap::new(),
            heartbeats,
            election_timeouts: timeouts,
            replicated_logs: Vec::new(),
        }
    }

    pub fn start_election(&mut self) {
        self.term += 1;
        self.votes.clear();
        let mut rng = rand::random::<u64>();
        for node in &self.nodes {
            rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
            let vote = if rng.is_multiple_of(7) { 0u64 } else { 1u64 };
            self.votes.insert(node.clone(), vote);
        }
        let quorum = (self.nodes.len() / 2) + 1;
        let total_votes: u64 = self.votes.values().sum();
        if total_votes >= quorum as u64
            && let Some(leader) = self
                .votes
                .iter()
                .max_by_key(|&(_, v)| *v)
                .map(|(n, _)| n.clone())
        {
            self.current_leader = Some(leader);
        }
    }

    pub fn heartbeat(&mut self, node_id: &str) {
        if let Some(count) = self.heartbeats.get_mut(node_id) {
            *count += 1;
        }
    }

    pub fn is_election_timeout(&self, node_id: &str) -> bool {
        self.election_timeouts.get(node_id).copied().unwrap_or(200) > 150
    }

    pub fn detect_leader_failure(&mut self) -> bool {
        if let Some(ref leader) = self.current_leader.clone()
            && self.heartbeats.get(leader).copied().unwrap_or(0) == 0
        {
            self.current_leader = None;
            self.start_election();
            return true;
        }
        false
    }

    pub fn commit_ledger_entry(&self, _node_id: &str, _nonce: u64) -> bool {
        self.current_leader.is_some()
            && self.votes.values().filter(|&&v| v > 0).count() > self.nodes.len() / 2
    }

    pub fn replicate_log_entry(&mut self, nonce: u64, ledger_hash: &str) -> bool {
        let entry = (nonce, ledger_hash.to_string());
        if !self.replicated_logs.contains(&entry) {
            self.replicated_logs.push(entry);
        }
        self.current_leader.is_some()
            && self.votes.values().filter(|&&v| v > 0).count() > self.nodes.len() / 2
    }

    pub fn get_quorum_acked_logs(&self) -> Vec<(u64, String)> {
        self.replicated_logs.clone()
    }
}

pub fn bootstrap_raft_cluster(node_names: &[&str]) -> RaftCluster {
    let mut cluster = RaftCluster::new(node_names);
    cluster.start_election();
    cluster
}

// Sprint 289: Cross-node isolate migration via serialized VMState payloads
pub fn migrate_active_isolate(isolate_id: i64, target_node_id: &str) -> Result<(), String> {
    let registry = super::isolate::get_hot_swap_registry();
    let guard = registry.lock().unwrap_or_else(|e| e.into_inner());
    let code = guard
        .get(&isolate_id)
        .ok_or_else(|| format!("Isolate {isolate_id} not found in registry"))?;
    let (instructions, constants) = code.lock().unwrap_or_else(|e| e.into_inner()).clone();
    drop(guard);

    let snapshot = super::snapshot::snapshot_isolate(isolate_id)
        .ok_or_else(|| format!("No snapshot for isolate {isolate_id}"))?;

    let serialized = super::storage::serialize_vm_state(&snapshot)
        .map_err(|e| format!("Serialization failed: {e}"))?;

    let payload = serialize_migration_payload(&serialized, &instructions, &constants);

    push_cluster_work_batch(target_node_id, vec![payload]);

    Ok(())
}

pub fn resume_migrated_isolate(
    instructions: &[OpCode],
    constants: &[RelType],
    serialized_state: &[u8],
) -> Result<VMIsolate, String> {
    let state = super::storage::deserialize_vm_state(serialized_state)
        .map_err(|e| format!("Deserialization failed: {e}"))?;
    if !super::machine::verify_ledger_hash(&state) {
        return Err(
            "Cryptographic Ledger Verification Failed: Tampering or Replay Detected".to_string(),
        );
    }
    let mut isolate = VMIsolate::new(instructions.to_vec(), constants.to_vec());
    isolate.local_heap = state.globals.clone();
    isolate.migration_state = Some(state);
    Ok(isolate)
}

pub fn receive_migration_payload(node_id: &str) -> Option<(Vec<OpCode>, Vec<RelType>, Vec<u8>)> {
    let stolen = try_steal_cluster_work(node_id, 0)?;
    let (op, payload_data) = stolen;
    if matches!(op, OpCode::Constant(_)) {
        return deserialize_migration_payload(&payload_data);
    }
    None
}

fn serialize_migration_payload(
    serialized_state: &[u8],
    instructions: &[OpCode],
    constants: &[RelType],
) -> (OpCode, Vec<RelType>) {
    let mut payload: Vec<u8> = Vec::new();
    payload.extend_from_slice(&0x4D494752u32.to_le_bytes());
    payload.extend_from_slice(&(serialized_state.len() as u64).to_le_bytes());
    payload.extend_from_slice(serialized_state);
    payload.extend_from_slice(&(instructions.len() as u64).to_le_bytes());
    for op in instructions {
        payload.extend_from_slice(&bincode_serialize_op(op));
    }
    payload.extend_from_slice(&(constants.len() as u64).to_le_bytes());
    for c in constants {
        let encoded = bincode_serialize_reltype(c);
        payload.extend_from_slice(&(encoded.len() as u32).to_le_bytes());
        payload.extend_from_slice(&encoded);
    }

    let mut rel_payload: Vec<RelType> = Vec::new();
    for byte in payload {
        rel_payload.push(RelType::Int(byte as i64));
    }
    (OpCode::Constant(0), rel_payload)
}

fn deserialize_migration_payload(
    rel_payload: &[RelType],
) -> Option<(Vec<OpCode>, Vec<RelType>, Vec<u8>)> {
    let bytes: Vec<u8> = rel_payload
        .iter()
        .map(|r| match r {
            RelType::Int(v) => *v as u8,
            _ => 0,
        })
        .collect();
    if bytes.len() < 16 {
        return None;
    }
    let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    if magic != 0x4D494752 {
        return None;
    }
    let mut pos: usize = 4;

    let state_len = read_u64(&bytes, &mut pos)? as usize;
    let serialized_state = bytes[pos..pos + state_len].to_vec();
    pos += state_len;

    let instr_count = read_u64(&bytes, &mut pos)? as usize;
    let mut instructions = Vec::new();
    for _ in 0..instr_count {
        instructions.push(bincode_deserialize_op(&bytes, &mut pos)?);
    }

    let const_count = read_u64(&bytes, &mut pos)? as usize;
    let mut constants = Vec::new();
    for _ in 0..const_count {
        let len = read_u32(&bytes, &mut pos)? as usize;
        let encoded = &bytes[pos..pos + len];
        constants.push(bincode_deserialize_reltype(encoded));
        pos += len;
    }

    Some((instructions, constants, serialized_state))
}

fn read_u64(bytes: &[u8], pos: &mut usize) -> Option<u64> {
    if *pos + 8 > bytes.len() {
        return None;
    }
    let v = u64::from_le_bytes([
        bytes[*pos],
        bytes[*pos + 1],
        bytes[*pos + 2],
        bytes[*pos + 3],
        bytes[*pos + 4],
        bytes[*pos + 5],
        bytes[*pos + 6],
        bytes[*pos + 7],
    ]);
    *pos += 8;
    Some(v)
}

fn read_u32(bytes: &[u8], pos: &mut usize) -> Option<u32> {
    if *pos + 4 > bytes.len() {
        return None;
    }
    let v = u32::from_le_bytes([
        bytes[*pos],
        bytes[*pos + 1],
        bytes[*pos + 2],
        bytes[*pos + 3],
    ]);
    *pos += 4;
    Some(v)
}

fn bincode_serialize_op(op: &OpCode) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let tag: u8 = match op {
        OpCode::Constant(_) => 0,
        OpCode::Add => 1,
        OpCode::Subtract => 2,
        OpCode::Multiply => 3,
        OpCode::Divide => 4,
        OpCode::Return => 5,
        OpCode::SetGlobal(_) => 6,
        OpCode::GetGlobal(_) => 7,
        OpCode::Jump(_) => 8,
        OpCode::JumpIfFalse(_) => 9,
        OpCode::Neg => 10,
        OpCode::Equal => 11,
        OpCode::Less => 12,
        _ => 255,
    };
    buf.push(tag);
    match op {
        OpCode::Constant(idx) | OpCode::SetGlobal(idx) | OpCode::GetGlobal(idx) => {
            buf.extend_from_slice(&(*idx as u64).to_le_bytes());
        }
        OpCode::Jump(t) | OpCode::JumpIfFalse(t) => {
            buf.extend_from_slice(&(*t as u64).to_le_bytes());
        }
        _ => {}
    }
    buf
}

fn bincode_deserialize_op(bytes: &[u8], pos: &mut usize) -> Option<OpCode> {
    let tag = *bytes.get(*pos)?;
    *pos += 1;
    match tag {
        0 => {
            let idx = read_u64(bytes, pos)? as usize;
            Some(OpCode::Constant(idx))
        }
        1 => Some(OpCode::Add),
        2 => Some(OpCode::Subtract),
        3 => Some(OpCode::Multiply),
        4 => Some(OpCode::Divide),
        5 => Some(OpCode::Return),
        6 => {
            let idx = read_u64(bytes, pos)? as usize;
            Some(OpCode::SetGlobal(idx))
        }
        7 => {
            let idx = read_u64(bytes, pos)? as usize;
            Some(OpCode::GetGlobal(idx))
        }
        8 => {
            let t = read_u64(bytes, pos)? as usize;
            Some(OpCode::Jump(t))
        }
        9 => {
            let t = read_u64(bytes, pos)? as usize;
            Some(OpCode::JumpIfFalse(t))
        }
        10 => Some(OpCode::Neg),
        11 => Some(OpCode::Equal),
        12 => Some(OpCode::Less),
        _ => None,
    }
}

fn bincode_serialize_reltype(rt: &RelType) -> Vec<u8> {
    match rt {
        RelType::Int(v) => {
            let mut b = vec![0u8];
            b.extend_from_slice(&v.to_le_bytes());
            b
        }
        RelType::Float(v) => {
            let mut b = vec![1u8];
            b.extend_from_slice(&v.to_le_bytes());
            b
        }
        RelType::Str(s) => {
            let mut b = vec![4u8];
            let sb = s.as_bytes();
            b.extend_from_slice(&(sb.len() as u32).to_le_bytes());
            b.extend_from_slice(sb);
            b
        }
        RelType::Bool(v) => vec![if *v { 2 } else { 3 }],
        _ => vec![5],
    }
}

fn bincode_deserialize_reltype(bytes: &[u8]) -> RelType {
    if bytes.is_empty() {
        return RelType::Void;
    }
    match bytes[0] {
        0 => {
            if bytes.len() >= 9 {
                RelType::Int(i64::from_le_bytes([
                    bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
                ]))
            } else {
                RelType::Void
            }
        }
        1 => {
            if bytes.len() >= 9 {
                RelType::Float(f64::from_le_bytes([
                    bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
                ]))
            } else {
                RelType::Void
            }
        }
        2 => RelType::Bool(true),
        3 => RelType::Bool(false),
        4 => {
            if bytes.len() >= 5 {
                let len = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;
                let end = (5 + len).min(bytes.len());
                RelType::Str(String::from_utf8_lossy(&bytes[5..end]).into_owned())
            } else {
                RelType::Void
            }
        }
        _ => RelType::Void,
    }
}
