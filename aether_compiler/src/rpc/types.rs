use std::collections::{HashSet, VecDeque};

use knoten_core_types::ast::Node;
use knoten_core_types::opcode::OpCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::executor::RelType;
use crate::vm::compiler::Compiler;
pub use crate::vm::machine::HmrReport;
use crate::vm::machine::{VM, VmEvent, VmExecutionState};

pub const KNC_PROTOCOL_VERSION: &str = "v2.24.4";
pub const MAX_CLOCK_DRIFT_SECS: u64 = 300;
pub const MAX_REPLAY_WINDOW_SECS: u64 = 60;
pub const MAX_ZERO_TRUST_WINDOW_SECS: u64 = 30;
pub const MAX_TASK_QUEUE_DEPTH: usize = 10_000;
pub const MAX_SYNC_ENTRIES: usize = 10_000;
pub const MAX_VALUE_SIZE_BYTES: usize = 65_536;
pub const MAX_STORE_KEYS: usize = 100_000;
pub const MAX_NONCE_CACHE_CAPACITY: usize = 10_000;
pub const MAX_BODY_BYTES: usize = 1_048_576;
pub const MAX_WS_PAYLOAD: usize = 1_048_576;
pub const MAX_PARAM_STRING_LEN: usize = 256;

/// Validates string parameter length limits against denial-of-service allocations.
pub fn validate_param_string_len(val: &str) -> Result<(), String> {
    if val.len() > MAX_PARAM_STRING_LEN {
        Err(format!(
            "Parameter exceeds maximum length limit ({} bytes)",
            MAX_PARAM_STRING_LEN
        ))
    } else {
        Ok(())
    }
}

/// Checks whether a timestamp is in the future beyond acceptable clock drift.
pub fn is_future_timestamp(ts: u64) -> bool {
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let ts_secs = if ts > 10_000_000_000 { ts / 1000 } else { ts };
    ts_secs > now_secs.saturating_add(MAX_CLOCK_DRIFT_SECS)
}

/// JSON-RPC 2.0 Request Payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 Response Payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 Error Payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
            id,
        }
    }
}

/// Bounded Nonce Cache with LRU eviction and automatic TTL cleanup.
#[derive(Debug, Clone, Default)]
pub struct NonceCache {
    set: HashSet<String>,
    queue: VecDeque<(String, u64)>,
}

impl NonceCache {
    pub fn new() -> Self {
        Self {
            set: HashSet::new(),
            queue: VecDeque::new(),
        }
    }

    pub fn insert(&mut self, nonce_entry: String, timestamp: u64) -> bool {
        if self.set.contains(&nonce_entry) {
            return false;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Evict expired entries older than 30s or when exceeding capacity limit
        while let Some((_, ts)) = self.queue.front() {
            let ts_secs = if *ts > 10_000_000_000 { ts / 1000 } else { *ts };
            if now.saturating_sub(ts_secs) > MAX_ZERO_TRUST_WINDOW_SECS
                || self.set.len() >= MAX_NONCE_CACHE_CAPACITY
            {
                if let Some((evicted, _)) = self.queue.pop_front() {
                    self.set.remove(&evicted);
                }
            } else {
                break;
            }
        }

        self.set.insert(nonce_entry.clone());
        self.queue.push_back((nonce_entry, timestamp));
        true
    }

    pub fn contains(&self, nonce_entry: &str) -> bool {
        self.set.contains(nonce_entry)
    }

    pub fn len(&self) -> usize {
        self.set.len()
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }
}

/// Thread-safe RPC Session State.
#[derive(Default)]
pub struct RpcSession {
    pub vm: VM,
    pub instructions: Vec<OpCode>,
    pub constants: Vec<RelType>,
    pub events: Vec<VmEvent>,
}

impl RpcSession {
    pub fn hot_reload_code(&mut self, new_ast: &Node) -> Result<HmrReport, String> {
        // 1. Pre-compilation validation (Transactional Safety)
        let (new_instructions, new_constants) = Compiler::compile(new_ast)?;

        // 2. Scoping Check: Reject reload if VM is actively executing
        if self.vm.execution_state == VmExecutionState::Running {
            return Err(
                "ERR_HMR_ACTIVE_EXECUTION: Cannot perform hot-reload on an active execution isolate"
                    .to_string(),
            );
        }

        let prev_len = self.instructions.len();
        let new_len = new_instructions.len();
        let preserved_vars = self.vm.globals.len();

        // 3. State Preservation & Bytecode Swapping
        self.instructions = new_instructions;
        self.constants = new_constants;
        self.vm.ip = 0;
        self.vm.stack.clear();
        self.vm.frames.clear();
        self.vm.execution_state = VmExecutionState::Ready;

        Ok(HmrReport {
            reloaded: true,
            previous_bytecode_len: prev_len,
            new_bytecode_len: new_len,
            preserved_variables: preserved_vars,
        })
    }
}

fn default_peer_status() -> String {
    "Active".to_string()
}

/// Mesh Peer Topology Information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshPeer {
    pub node_id: String,
    pub address: String,
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub last_seen: u64,
    #[serde(default)]
    pub latency_ms: u64,
    #[serde(default = "default_peer_status")]
    pub status: String,
}
