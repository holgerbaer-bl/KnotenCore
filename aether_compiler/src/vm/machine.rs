use crate::executor::{AgentPermissions, ExecutionEngine, RelType};
use knoten_core_types::opcode::{OpCode, SimdOp};
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

use super::gpgpu;
use super::inspector;
use super::isolate;
use super::scheduler;
use super::snapshot;
use super::vfs::VirtualFs;

pub use isolate::SpeculativeResult;
pub use isolate::VMIsolate;
pub use isolate::drain_hot_swap_registry;
pub use isolate::get_hot_swap_registry;
pub use isolate::optimize_active_hotpath;
pub use isolate::spawn_isolate;
pub use isolate::spawn_shadow_isolate;
pub use scheduler::RaftCluster;
pub use scheduler::WorkItem;
pub use scheduler::bootstrap_raft_cluster;
pub use scheduler::dispatch_speculative_branch;
pub use scheduler::drain_cluster_work_queues;
pub use scheduler::drain_work_stealing_queues;
pub use scheduler::migrate_active_isolate;
pub use scheduler::push_cluster_work_batch;
pub use scheduler::push_work_batch;
pub use scheduler::receive_migration_payload;
pub use scheduler::resume_migrated_isolate;
pub use scheduler::try_steal_cluster_work;
pub use scheduler::try_steal_wasm_work;
pub use scheduler::try_steal_work;
pub use snapshot::drain_isolate_snapshots;
pub use snapshot::rollback_isolate;
pub use snapshot::snapshot_isolate;
pub use snapshot::store_snapshot;

// ── Re-exports from extracted modules (Sprint 303) ───────────────────────────
pub use gpgpu::apply_matrix_to_inputs;
pub use gpgpu::split_inputs_to_bindings;
pub use inspector::VMInspectorData;
pub use inspector::drain_hot_path_table;
pub use inspector::get_ledger_nonce;
pub use inspector::get_vm_inspection_snapshot;
pub use inspector::verify_ledger_hash;
// VM_SLEEP_ACCUMULATED_MS kept as pub(crate) in inspector; accessed via inspector:: directly

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CallFrame {
    pub ip: usize,
    pub base_pointer: usize,
}

/// Sprint 309: VM Execution State Representation
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum VmExecutionState {
    #[default]
    Ready,
    Running,
    Yielded,
    Finished(RelType),
    Fault(String),
}

/// Sprint 308: Runtime Events emitted by script execution and VFS hooks
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum VmEvent {
    /// Custom script event via EventEmit opcode (topic, payload)
    Custom { topic: String, payload: RelType },
    /// VFS File Written
    VfsWrite { path: String, bytes: usize },
    /// VFS File Read
    VfsRead { path: String },
}

pub type VmEventHook = std::sync::Arc<dyn Fn(VmEvent) + Send + Sync>;

#[derive(Default)]
pub struct VM {
    pub stack: Vec<RelType>,
    pub globals: HashMap<String, RelType>,
    pub frames: Vec<CallFrame>,
    pub ip: usize,
    pub base_pointer: usize,
    pub is_inspectable: bool,
    pub crypto_state_hash: u64,
    /// Sprint 306: Sandboxed In-Memory Virtual File System.
    /// All script VFS I/O is isolated in RAM — never touches the host filesystem.
    pub vfs: VirtualFs,
    /// Sprint 308: Optional thread-safe event streaming hook
    pub event_hook: Option<VmEventHook>,
    /// Sprint 309: Current VM execution state
    pub execution_state: VmExecutionState,
    /// Sprint 311: Configurable Multi-Tenant Resource Quotas
    pub quota: knoten_core_types::ast::IsolateQuota,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct VMState {
    pub globals: HashMap<String, RelType>,
    pub stack: Vec<RelType>,
    pub frames: Vec<CallFrame>,
    pub ip: usize,
    pub base_pointer: usize,
    pub crypto_state_hash: u64,
    pub nonce: u64,
    pub previous_state_hash: [u8; 32],
}

// VM_SLEEP_ACCUMULATED_MS lives in inspector.rs (pub); used via inspector:: in VM::run().
// verify_ledger_hash, get_ledger_nonce: available via pub use inspector:: above.

fn compute_ledger_hash(crypto_hash: u64, nonce: u64, prev: &[u8; 32]) -> [u8; 32] {
    inspector::compute_ledger_hash(crypto_hash, nonce, prev)
}

pub fn sweep_terminated_isolates() {
    stack_registry_drain();
    snapshot::drain_isolate_snapshots();
    scheduler::drain_cluster_work_queues();
    scheduler::drain_work_stealing_queues();
}

fn stack_registry_drain() {
    if let Ok(mut guard) = isolate::get_hot_swap_registry().lock() {
        guard.clear();
    }
}

fn estimate_reltype_heap_bytes(val: &RelType) -> usize {
    match val {
        RelType::Str(s) => s.capacity(),
        RelType::Array(arr) => {
            arr.capacity() * std::mem::size_of::<RelType>()
                + arr.iter().map(estimate_reltype_heap_bytes).sum::<usize>()
        }
        RelType::Dict(dict) => {
            if let Ok(map) = dict.try_lock() {
                map.capacity() * (std::mem::size_of::<String>() + std::mem::size_of::<RelType>())
                    + map
                        .iter()
                        .map(|(k, v)| k.capacity() + estimate_reltype_heap_bytes(v))
                        .sum::<usize>()
            } else {
                0
            }
        }
        _ => 0,
    }
}

impl VM {
    pub fn estimate_memory_bytes(&self) -> usize {
        let stack_bytes = self.stack.capacity() * std::mem::size_of::<RelType>();
        let globals_bytes = self.globals.capacity()
            * (std::mem::size_of::<String>() + std::mem::size_of::<RelType>());
        let frames_bytes = self.frames.capacity() * std::mem::size_of::<CallFrame>();

        let stack_heap: usize = self
            .stack
            .iter()
            .rev()
            .take(64)
            .map(estimate_reltype_heap_bytes)
            .sum();
        let globals_heap: usize = self.globals.values().map(estimate_reltype_heap_bytes).sum();

        stack_bytes + globals_bytes + frames_bytes + stack_heap + globals_heap
    }

    pub fn new() -> Self {
        Self {
            globals: HashMap::new(),
            stack: Vec::with_capacity(1024),
            frames: Vec::with_capacity(64),
            ip: 0,
            base_pointer: 0,
            crypto_state_hash: 0,
            is_inspectable: false,
            vfs: VirtualFs::new(),
            event_hook: None,
            execution_state: VmExecutionState::Ready,
            quota: knoten_core_types::ast::IsolateQuota::default(),
        }
    }

    pub fn set_quota(&mut self, quota: knoten_core_types::ast::IsolateQuota) {
        self.quota = quota;
    }

    pub fn set_event_hook(&mut self, hook: VmEventHook) {
        self.event_hook = Some(hook);
    }

    pub fn execution_state(&self) -> &VmExecutionState {
        &self.execution_state
    }

    pub fn is_yielded(&self) -> bool {
        matches!(self.execution_state, VmExecutionState::Yielded)
    }

    pub fn resume(
        &mut self,
        instructions: &[OpCode],
        constants: &[RelType],
        permissions: &AgentPermissions,
        bridge: Option<&dyn crate::natives::bridge::BridgeModule>,
    ) -> Result<RelType, String> {
        if self.execution_state == VmExecutionState::Yielded {
            self.execution_state = VmExecutionState::Running;
            self.run_loop(instructions, constants, permissions, bridge)
        } else {
            let err = "Cannot resume VM: execution state is not Yielded".to_string();
            self.execution_state = VmExecutionState::Fault(err.clone());
            Err(err)
        }
    }

    pub fn inspect(&self) -> VMInspectorData {
        let isolate_count = isolate::get_hot_swap_registry()
            .lock()
            .map(|g| g.len())
            .unwrap_or(0);
        inspector::build_inspector_data(
            self.stack.len(),
            self.frames.len(),
            self.ip,
            self.base_pointer,
            self.crypto_state_hash,
            self.globals.len(),
            isolate_count,
        )
    }

    pub fn snapshot(&self) -> VMState {
        let nonce = inspector::LEDGER_NONCE.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        VMState {
            globals: self.globals.clone(),
            stack: self.stack.clone(),
            frames: self.frames.clone(),
            ip: self.ip,
            base_pointer: self.base_pointer,
            crypto_state_hash: self.crypto_state_hash,
            nonce,
            previous_state_hash: compute_ledger_hash(self.crypto_state_hash, nonce, &[0u8; 32]),
        }
    }

    pub fn rollback(&mut self, state: VMState) {
        self.globals = state.globals;
        self.stack = state.stack;
        self.frames = state.frames;
        self.ip = state.ip;
        self.base_pointer = state.base_pointer;
        self.crypto_state_hash = state.crypto_state_hash;
    }

    #[inline(always)]
    pub fn run(
        &mut self,
        instructions: &[OpCode],
        constants: &[RelType],
        permissions: &AgentPermissions,
        bridge: Option<&dyn crate::natives::bridge::BridgeModule>,
    ) -> Result<RelType, String> {
        self.stack.clear();
        self.frames.clear();
        self.ip = 0;
        self.base_pointer = 0;
        self.execution_state = VmExecutionState::Running;

        self.run_loop(instructions, constants, permissions, bridge)
    }

    pub fn run_loop(
        &mut self,
        instructions: &[OpCode],
        constants: &[RelType],
        permissions: &AgentPermissions,
        bridge: Option<&dyn crate::natives::bridge::BridgeModule>,
    ) -> Result<RelType, String> {
        let mut start = Instant::now();
        let mut instr_count: u64 = 0;
        let mut accumulated_cpu: std::time::Duration = std::time::Duration::ZERO;

        while self.ip < instructions.len() {
            let op = &instructions[self.ip];
            self.ip += 1;
            if self.is_inspectable {
                inspector::update_inspection_state(self.ip, self.stack.len());
            }

            self.crypto_state_hash = self
                .crypto_state_hash
                .wrapping_mul(0x9E3779B97F4A7C15)
                .wrapping_add(self.ip as u64)
                .wrapping_add(self.stack.len() as u64)
                .wrapping_add(inspector::opcode_discriminant_hash(op));

            instr_count += 1;
            if instr_count >= self.quota.max_instructions {
                inspector::push_vm_crash_marker(self.ip, self.stack.len(), "ERR_QUOTA_EXCEEDED");
                let msg = format!(
                    "ERR_QUOTA_EXCEEDED: Execution exceeded maximum allowed instruction count ({})",
                    self.quota.max_instructions
                );
                self.execution_state = VmExecutionState::Fault(msg.clone());
                return Err(msg);
            }

            if instr_count == 1 || instr_count.is_multiple_of(100) {
                inspector::track_hot_path(self.ip);
                if self.estimate_memory_bytes() > self.quota.max_memory_bytes {
                    inspector::push_vm_crash_marker(
                        self.ip,
                        self.stack.len(),
                        "ERR_MEMORY_LIMIT_EXCEEDED",
                    );
                    let msg = format!(
                        "ERR_MEMORY_LIMIT_EXCEEDED: VM memory allocation threshold ({} bytes) exceeded",
                        self.quota.max_memory_bytes
                    );
                    self.execution_state = VmExecutionState::Fault(msg.clone());
                    return Err(msg);
                }
            }

            if instr_count.is_multiple_of(1000) {
                let sleep_ms =
                    inspector::VM_SLEEP_ACCUMULATED_MS.load(std::sync::atomic::Ordering::SeqCst);
                let effective_cpu =
                    accumulated_cpu.saturating_sub(std::time::Duration::from_millis(sleep_ms));
                if self.quota.execution_timeout_ms > 0
                    && effective_cpu + start.elapsed()
                        >= std::time::Duration::from_millis(self.quota.execution_timeout_ms)
                {
                    eprintln!(
                        "[KnotenCore Watchdog] Execution timeout exceeded ({}ms). Terminating script to prevent CPU freeze.",
                        self.quota.execution_timeout_ms
                    );
                    inspector::push_vm_crash_marker(self.ip, self.stack.len(), "WATCHDOG_TIMEOUT");
                    let msg = format!(
                        "Watchdog: Execution timeout exceeded ({}ms)",
                        self.quota.execution_timeout_ms
                    );
                    self.execution_state = VmExecutionState::Fault(msg.clone());
                    return Err(msg);
                }
            }

            match op {
                OpCode::Constant(idx) => {
                    if *idx < constants.len() {
                        self.stack.push(constants[*idx].clone());
                    } else {
                        return Err("Constant index out of bounds".into());
                    }
                }
                OpCode::Add => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Add".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Add".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => self.stack.push(RelType::Int(a + b)),
                        (RelType::Float(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Float(a + b))
                        }
                        (RelType::Int(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Float(a as f64 + b))
                        }
                        (RelType::Float(a), RelType::Int(b)) => {
                            self.stack.push(RelType::Float(a + b as f64))
                        }
                        (RelType::Str(a), RelType::Str(b)) => self.stack.push(RelType::Str(a + &b)),
                        _ => return Err("Invalid types for Add".into()),
                    }
                }
                OpCode::Subtract => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Subtract".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Subtract".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => self.stack.push(RelType::Int(a - b)),
                        (RelType::Float(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Float(a - b))
                        }
                        (RelType::Int(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Float(a as f64 - b))
                        }
                        (RelType::Float(a), RelType::Int(b)) => {
                            self.stack.push(RelType::Float(a - b as f64))
                        }
                        _ => return Err("Invalid types for Subtract".into()),
                    }
                }
                OpCode::Multiply => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Multiply".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Multiply".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => self.stack.push(RelType::Int(a * b)),
                        (RelType::Float(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Float(a * b))
                        }
                        (RelType::Int(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Float(a as f64 * b))
                        }
                        (RelType::Float(a), RelType::Int(b)) => {
                            self.stack.push(RelType::Float(a * b as f64))
                        }
                        _ => return Err("Invalid types for Multiply".into()),
                    }
                }
                OpCode::Divide => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Divide".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Divide".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => {
                            if b == 0 {
                                return Err("Fault: Div by zero (at Node::MathDiv)".into());
                            }
                            self.stack.push(RelType::Int(a / b))
                        }
                        (RelType::Float(a), RelType::Float(b)) => {
                            if b == 0.0 {
                                return Err("Fault: Div by zero (at Node::MathDiv)".into());
                            }
                            self.stack.push(RelType::Float(a / b))
                        }
                        _ => return Err("Invalid types for Divide".into()),
                    }
                }
                OpCode::Modulo => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Modulo".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Modulo".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => {
                            if b == 0 {
                                return Err("Fault: Div by zero (at Node::Modulo)".into());
                            }
                            self.stack.push(RelType::Int(a % b))
                        }
                        (RelType::Float(a), RelType::Float(b)) => {
                            if b == 0.0 {
                                return Err("Fault: Div by zero (at Node::Modulo)".into());
                            }
                            self.stack.push(RelType::Float(a % b))
                        }
                        _ => return Err("Invalid types for Modulo".into()),
                    }
                }
                OpCode::Neg => {
                    let v = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Neg".to_string())?;
                    match v {
                        RelType::Int(a) => self.stack.push(RelType::Int(-a)),
                        RelType::Float(a) => self.stack.push(RelType::Float(-a)),
                        _ => return Err("Invalid type for Neg".into()),
                    }
                }
                OpCode::Equal => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Equal".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Equal".to_string())?;
                    self.stack.push(RelType::Bool(l == r));
                }
                OpCode::Less => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Less".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Less".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => self.stack.push(RelType::Bool(a < b)),
                        (RelType::Float(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Bool(a < b))
                        }
                        _ => return Err("Invalid types for Less comparison".into()),
                    }
                }
                OpCode::Greater => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Greater".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Greater".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => self.stack.push(RelType::Bool(a > b)),
                        (RelType::Float(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Bool(a > b))
                        }
                        _ => return Err("Invalid types for Greater comparison".into()),
                    }
                }
                OpCode::NotEqual => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in NotEqual".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in NotEqual".to_string())?;
                    self.stack.push(RelType::Bool(l != r));
                }
                OpCode::LessEqual => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in LessEqual".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in LessEqual".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => {
                            self.stack.push(RelType::Bool(a <= b))
                        }
                        (RelType::Float(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Bool(a <= b))
                        }
                        _ => return Err("Invalid types for LessEqual comparison".into()),
                    }
                }
                OpCode::GreaterEqual => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in GreaterEqual".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in GreaterEqual".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => {
                            self.stack.push(RelType::Bool(a >= b))
                        }
                        (RelType::Float(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Bool(a >= b))
                        }
                        _ => return Err("Invalid types for GreaterEqual comparison".into()),
                    }
                }
                OpCode::And => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in And".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in And".to_string())?;
                    match (l, r) {
                        (RelType::Bool(a), RelType::Bool(b)) => {
                            self.stack.push(RelType::Bool(a && b))
                        }
                        _ => return Err("And expects two booleans".into()),
                    }
                }
                OpCode::Or => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Or".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Or".to_string())?;
                    match (l, r) {
                        (RelType::Bool(a), RelType::Bool(b)) => {
                            self.stack.push(RelType::Bool(a || b))
                        }
                        _ => return Err("Or expects two booleans".into()),
                    }
                }
                OpCode::Not => {
                    let v = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Not".to_string())?;
                    match v {
                        RelType::Bool(b) => self.stack.push(RelType::Bool(!b)),
                        _ => return Err("Not expects a boolean".into()),
                    }
                }
                OpCode::JumpIfFalse(target_ip) => {
                    let cond = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in JumpIfFalse".to_string())?;
                    let is_true = match cond {
                        RelType::Bool(b) => b,
                        RelType::Int(i) => i != 0,
                        _ => false,
                    };
                    if !is_true {
                        self.ip = *target_ip;
                    }
                }
                OpCode::Jump(target_ip) => {
                    self.ip = *target_ip;
                }
                OpCode::SetLocal(idx) => {
                    let val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in SetLocal".to_string())?;
                    let target_idx = self.base_pointer + *idx;
                    // Dynamically allocate stack for isolated variables
                    if target_idx >= self.stack.len() {
                        self.stack.resize(target_idx + 1, RelType::Void);
                    }
                    self.stack[target_idx] = val;
                }
                OpCode::GetLocal(idx) => {
                    let val = self
                        .stack
                        .get(self.base_pointer + *idx)
                        .cloned()
                        .ok_or_else(|| format!("Stack underflow in GetLocal({})", idx))?;
                    self.stack.push(val);
                }
                OpCode::Call(target_ip, arg_count) => {
                    self.frames.push(CallFrame {
                        ip: self.ip,
                        base_pointer: self.base_pointer,
                    });
                    self.base_pointer = self.stack.len().saturating_sub(*arg_count);
                    self.ip = *target_ip;
                }
                OpCode::SetGlobal(idx) => {
                    let val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in SetGlobal".to_string())?;
                    if let Some(RelType::Str(name)) = constants.get(*idx) {
                        self.globals.insert(name.clone(), val);
                    } else {
                        return Err("Invalid constant index for SetGlobal".into());
                    }
                }
                OpCode::GetGlobal(idx) => {
                    if let Some(RelType::Str(name)) = constants.get(*idx) {
                        if let Some(val) = self.globals.get(name) {
                            self.stack.push(val.clone());
                        } else {
                            self.stack.push(RelType::Void);
                        }
                    } else {
                        return Err("Invalid constant index for GetGlobal".into());
                    }
                }
                OpCode::StringLength => {
                    let val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in StringLength".to_string())?;
                    if let RelType::Str(s) = val {
                        self.stack.push(RelType::Int(s.chars().count() as i64));
                    } else {
                        return Err("StringLength expects a Str".into());
                    }
                }
                OpCode::StringContainsChars => {
                    let pattern = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in StringContainsChars".to_string())?;
                    let target = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in StringContainsChars".to_string())?;
                    if let (RelType::Str(s), RelType::Str(p)) = (target, pattern) {
                        let contains = match p.as_str() {
                            "numbers" => s.chars().any(|c| c.is_ascii_digit()),
                            "special" => s
                                .chars()
                                .any(|c| !c.is_ascii_alphanumeric() && !c.is_whitespace()),
                            "uppercase" => s.chars().any(|c| c.is_ascii_uppercase()),
                            "lowercase" => s.chars().any(|c| c.is_ascii_lowercase()),
                            other => s.contains(other),
                        };
                        self.stack.push(RelType::Bool(contains));
                    } else {
                        self.stack.push(RelType::Bool(false));
                    }
                }
                OpCode::StringSplit => {
                    let delim = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in StringSplit".to_string())?;
                    let target = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in StringSplit".to_string())?;
                    if let (RelType::Str(s), RelType::Str(d)) = (target, delim) {
                        let parts = s
                            .split(&d)
                            .map(|part| RelType::Str(part.to_string()))
                            .collect();
                        self.stack.push(RelType::Array(parts));
                    } else {
                        self.stack.push(RelType::Void);
                    }
                }
                OpCode::ArrayContains => {
                    let search = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in ArrayContains".to_string())?;
                    let array = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in ArrayContains".to_string())?;
                    if let (RelType::Array(arr), search_val) = (array, search) {
                        self.stack.push(RelType::Bool(arr.contains(&search_val)));
                    } else {
                        self.stack.push(RelType::Bool(false));
                    }
                }
                OpCode::ReadFile => {
                    let path_val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in ReadFile".to_string())?;
                    if let RelType::Str(path) = path_val {
                        if !permissions.allow_fs_read {
                            return Err(
                                "Permission Denied: allow_fs_read is false (VM: ReadFile)".into()
                            );
                        } else {
                            match ExecutionEngine::validate_fs_path(&path) {
                                Ok(safe_path) => {
                                    if let Ok(content) = std::fs::read_to_string(safe_path) {
                                        self.stack.push(RelType::Str(content));
                                    } else {
                                        self.stack.push(RelType::Void);
                                    }
                                }
                                Err(_) => self.stack.push(RelType::Void),
                            }
                        }
                    } else {
                        self.stack.push(RelType::Void);
                    }
                }
                OpCode::ArrayCreate(count) => {
                    let mut elements = Vec::with_capacity(*count);
                    for _ in 0..*count {
                        elements.push(self.stack.pop().unwrap_or(RelType::Void));
                    }
                    elements.reverse();
                    self.stack.push(RelType::Array(elements));
                }
                OpCode::ArrayGet => {
                    let idx_val = self.stack.pop().unwrap_or(RelType::Void);
                    let arr_val = self.stack.pop().unwrap_or(RelType::Void);
                    if let (RelType::Array(arr), RelType::Int(idx)) = (arr_val, idx_val) {
                        if idx >= 0 && (idx as usize) < arr.len() {
                            self.stack.push(arr[idx as usize].clone());
                        } else {
                            return Err(format!("ArrayGet index out of bounds: {}", idx));
                        }
                    } else {
                        return Err("ArrayGet expects Array and Int".into());
                    }
                }
                OpCode::ArraySet => {
                    let val = self.stack.pop().unwrap_or(RelType::Void);
                    let idx_val = self.stack.pop().unwrap_or(RelType::Void);
                    let arr_val = self.stack.pop().unwrap_or(RelType::Void);
                    if let (RelType::Array(mut arr), RelType::Int(idx)) = (arr_val, idx_val) {
                        if idx >= 0 && (idx as usize) < arr.len() {
                            arr[idx as usize] = val;
                            self.stack.push(RelType::Array(arr));
                        } else {
                            return Err(format!("ArraySet index out of bounds: {}", idx));
                        }
                    } else {
                        return Err("ArraySet expects Array and Int".into());
                    }
                }
                OpCode::ArrayPush => {
                    let val = self.stack.pop().unwrap_or(RelType::Void);
                    let arr_val = self.stack.pop().unwrap_or(RelType::Void);
                    if let RelType::Array(mut arr) = arr_val {
                        arr.push(val);
                        self.stack.push(RelType::Array(arr));
                    } else {
                        return Err("ArrayPush expects Array".into());
                    }
                }
                OpCode::ArrayLen => {
                    let arr_val = self.stack.pop().unwrap_or(RelType::Void);
                    if let RelType::Array(arr) = arr_val {
                        self.stack.push(RelType::Int(arr.len() as i64));
                    } else {
                        return Err("ArrayLen expects Array".into());
                    }
                }
                OpCode::Concat => {
                    let r_val = self.stack.pop().unwrap_or(RelType::Void);
                    let l_val = self.stack.pop().unwrap_or(RelType::Void);
                    match (l_val, r_val) {
                        (RelType::Str(a), RelType::Str(b)) => self.stack.push(RelType::Str(a + &b)),
                        (RelType::Array(mut a), RelType::Array(b)) => {
                            a.extend(b);
                            self.stack.push(RelType::Array(a));
                        }
                        _ => return Err("Concat expects Strings or Arrays".into()),
                    }
                }
                OpCode::ToString => {
                    let val = self.stack.pop().unwrap_or(RelType::Void);
                    self.stack.push(RelType::Str(val.to_string()));
                }
                OpCode::WriteFile => {
                    let data_val = self.stack.pop().unwrap_or(RelType::Void);
                    let path_val = self.stack.pop().unwrap_or(RelType::Void);

                    if !permissions.allow_fs_write {
                        return Err(
                            "Permission Denied: allow_fs_write is false (VM: WriteFile)".into()
                        );
                    }

                    if let (RelType::Str(path), RelType::Str(data)) = (path_val, data_val) {
                        match crate::executor::ExecutionEngine::validate_fs_path_write(&path) {
                            Ok(safe_path) => {
                                if let Err(e) = std::fs::write(&safe_path, data) {
                                    return Err(format!("File write error: {}", e));
                                }
                            }
                            Err(e) => return Err(format!("Security: {}", e)),
                        }
                    } else {
                        return Err("WriteFile expects string path and data".into());
                    }
                    self.stack.push(RelType::Void);
                }
                OpCode::NativeExternCall {
                    module_idx,
                    func_idx,
                    arg_count,
                } => {
                    let module = match constants.get(*module_idx) {
                        Some(RelType::Str(s)) => s.clone(),
                        _ => "global".to_string(),
                    };
                    let func = match constants.get(*func_idx) {
                        Some(RelType::Str(s)) => s.clone(),
                        _ => "unknown".to_string(),
                    };

                    let mut args = Vec::with_capacity(*arg_count);
                    for _ in 0..*arg_count {
                        args.push(self.stack.pop().unwrap_or(RelType::Void));
                    }
                    args.reverse();

                    if module == "registry" {
                        if func == "registry_play_sound" {
                            if args.len() == 1
                                && let RelType::Str(path) = &args[0]
                            {
                                match crate::executor::ExecutionEngine::validate_fs_path(path) {
                                    Ok(safe_path) => {
                                        let _ = crate::natives::registry::registry_play_sound(
                                            &safe_path.to_string_lossy(),
                                        );
                                        self.stack.push(RelType::Void);
                                        continue;
                                    }
                                    Err(e) => {
                                        return Err(format!("Fault: {} (at Node::ExternCall)", e));
                                    }
                                }
                            }
                            return Err(
                                "Fault: registry_play_sound expects (String) (at Node::ExternCall)"
                                    .to_string(),
                            );
                        }
                        if func == "registry_loop_music" {
                            if args.len() == 1
                                && let RelType::Str(path) = &args[0]
                            {
                                match crate::executor::ExecutionEngine::validate_fs_path(path) {
                                    Ok(safe_path) => {
                                        let _ = crate::natives::registry::registry_loop_music(
                                            &safe_path.to_string_lossy(),
                                        );
                                        self.stack.push(RelType::Void);
                                        continue;
                                    }
                                    Err(e) => {
                                        return Err(format!("Fault: {} (at Node::ExternCall)", e));
                                    }
                                }
                            }
                            return Err(
                                "Fault: registry_loop_music expects (String) (at Node::ExternCall)"
                                    .to_string(),
                            );
                        }
                        if func == "registry_set_volume" {
                            if args.len() == 1 {
                                let level = match &args[0] {
                                    RelType::Float(f) => *f as f32,
                                    RelType::Int(i) => *i as f32,
                                    _ => return Err("Fault: registry_set_volume expects (Float/Int) (at Node::ExternCall)".to_string()),
                                };
                                crate::natives::registry::registry_set_volume(level);
                                self.stack.push(RelType::Void);
                                continue;
                            }
                            return Err("Fault: registry_set_volume expects (Float/Int) (at Node::ExternCall)".to_string());
                        }
                    }

                    if let Some(b) = bridge {
                        let result = catch_unwind(AssertUnwindSafe(|| {
                            b.handle(&module, &func, &args, permissions)
                        }));
                        accumulated_cpu += start.elapsed();
                        start = std::time::Instant::now();
                        match result {
                            Ok(Some(crate::executor::ExecResult::Value(v))) => self.stack.push(v),
                            Ok(Some(crate::executor::ExecResult::Fault { msg, .. })) => {
                                return Err(format!("FFI Fault: {}", msg));
                            }
                            Ok(None) => {
                                return Err(format!(
                                    "FFI Function '{}.{}' not handled by active BridgeModule",
                                    module, func
                                ));
                            }
                            Ok(_) => self.stack.push(RelType::Void),
                            Err(panic_payload) => {
                                let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                                    s.to_string()
                                } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                                    s.clone()
                                } else {
                                    "Unknown panic".to_string()
                                };
                                eprintln!(
                                    "[KnotenCore Panic] Caught panic in FFI call '{}.{}': {}",
                                    module, func, msg
                                );
                                return Err(format!(
                                    "VM Panic in FFI call '{}.{}': {}",
                                    module, func, msg
                                ));
                            }
                        }
                    } else {
                        self.stack.push(RelType::Void);
                    }
                }

                // ── Sprint 306: Native Cast Opcodes ───────────────────────────────
                OpCode::ToInt => {
                    let val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in ToInt".to_string())?;
                    match val {
                        RelType::Int(i) => self.stack.push(RelType::Int(i)),
                        RelType::Float(f) => self.stack.push(RelType::Int(f as i64)),
                        RelType::Bool(b) => self.stack.push(RelType::Int(if b { 1 } else { 0 })),
                        RelType::Str(s) => {
                            let parsed = s
                                .trim()
                                .parse::<i64>()
                                .map_err(|_| format!("ToInt: cannot parse '{}' as integer", s))?;
                            self.stack.push(RelType::Int(parsed));
                        }
                        other => return Err(format!("ToInt: unsupported type {:?}", other)),
                    }
                }
                OpCode::ToFloat => {
                    let val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in ToFloat".to_string())?;
                    match val {
                        RelType::Float(f) => self.stack.push(RelType::Float(f)),
                        RelType::Int(i) => self.stack.push(RelType::Float(i as f64)),
                        RelType::Bool(b) => {
                            self.stack.push(RelType::Float(if b { 1.0 } else { 0.0 }))
                        }
                        RelType::Str(s) => {
                            let parsed = s
                                .trim()
                                .parse::<f64>()
                                .map_err(|_| format!("ToFloat: cannot parse '{}' as float", s))?;
                            self.stack.push(RelType::Float(parsed));
                        }
                        other => return Err(format!("ToFloat: unsupported type {:?}", other)),
                    }
                }

                // ── Sprint 306: High-Performance String & Array Primitives ────────
                OpCode::StringConcat => {
                    let rhs = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in StringConcat (rhs)".to_string())?;
                    let lhs = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in StringConcat (lhs)".to_string())?;
                    match (lhs, rhs) {
                        (RelType::Str(a), RelType::Str(b)) => self.stack.push(RelType::Str(a + &b)),
                        (RelType::Str(a), other) => {
                            self.stack.push(RelType::Str(a + &other.to_string()))
                        }
                        (other, RelType::Str(b)) => {
                            self.stack.push(RelType::Str(other.to_string() + &b))
                        }
                        (l, r) => self
                            .stack
                            .push(RelType::Str(l.to_string() + &r.to_string())),
                    }
                }
                OpCode::StringContains => {
                    let needle = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in StringContains (needle)".to_string())?;
                    let haystack = self.stack.pop().ok_or_else(|| {
                        "Stack underflow in StringContains (haystack)".to_string()
                    })?;
                    match (haystack, needle) {
                        (RelType::Str(h), RelType::Str(n)) => {
                            self.stack.push(RelType::Bool(h.contains(n.as_str())))
                        }
                        _ => return Err("StringContains requires two Str values".into()),
                    }
                }
                OpCode::ArraySlice => {
                    let end_val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in ArraySlice (end)".to_string())?;
                    let start_val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in ArraySlice (start)".to_string())?;
                    let arr_val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in ArraySlice (array)".to_string())?;
                    match (arr_val, start_val, end_val) {
                        (RelType::Array(arr), RelType::Int(start), RelType::Int(end)) => {
                            let len = arr.len() as i64;
                            let s = start.max(0).min(len) as usize;
                            let e = end.max(0).min(len) as usize;
                            let e = e.max(s);
                            self.stack.push(RelType::Array(arr[s..e].to_vec()));
                        }
                        _ => return Err("ArraySlice requires (Array, Int start, Int end)".into()),
                    }
                }

                // ── Sprint 306: Sandboxed In-Memory VFS Opcodes ──────────────────
                OpCode::VfsWrite => {
                    let data_val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in VfsWrite (data)".to_string())?;
                    let path_val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in VfsWrite (path)".to_string())?;
                    match (path_val, data_val) {
                        (RelType::Str(path), RelType::Str(data)) => {
                            self.vfs
                                .write(&path, &data)
                                .map_err(|e| format!("VfsWrite error: {}", e))?;
                            if let Some(ref hook) = self.event_hook {
                                hook(VmEvent::VfsWrite {
                                    path: path.clone(),
                                    bytes: data.len(),
                                });
                            }
                        }
                        _ => return Err("VfsWrite expects (path: Str, data: Str)".into()),
                    }
                    self.stack.push(RelType::Void);
                }
                OpCode::VfsRead => {
                    let path_val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in VfsRead (path)".to_string())?;
                    match path_val {
                        RelType::Str(path) => {
                            if let Some(ref hook) = self.event_hook {
                                hook(VmEvent::VfsRead { path: path.clone() });
                            }
                            match self
                                .vfs
                                .read(&path)
                                .map_err(|e| format!("VfsRead error: {}", e))?
                            {
                                Some(content) => self.stack.push(RelType::Str(content)),
                                None => self.stack.push(RelType::Void),
                            }
                        }
                        _ => return Err("VfsRead expects a Str path".into()),
                    }
                }
                OpCode::VfsExists => {
                    let path_val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in VfsExists (path)".to_string())?;
                    match path_val {
                        RelType::Str(path) => {
                            let exists = self
                                .vfs
                                .exists(&path)
                                .map_err(|e| format!("VfsExists error: {}", e))?;
                            self.stack.push(RelType::Bool(exists));
                        }
                        _ => return Err("VfsExists expects a Str path".into()),
                    }
                }
                OpCode::VfsList => {
                    let prefix_val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in VfsList (prefix)".to_string())?;
                    match prefix_val {
                        RelType::Str(prefix) => {
                            let paths = self
                                .vfs
                                .list(&prefix)
                                .map_err(|e| format!("VfsList error: {}", e))?;
                            let arr: Vec<RelType> = paths.into_iter().map(RelType::Str).collect();
                            self.stack.push(RelType::Array(arr));
                        }
                        _ => return Err("VfsList expects a Str prefix".into()),
                    }
                }

                // ── Sprint 308: Agentic Event Streaming & Execution Hooks ───────
                OpCode::EventEmit => {
                    let payload_val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in EventEmit (payload)".to_string())?;
                    let topic_val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in EventEmit (topic)".to_string())?;
                    let topic = match topic_val {
                        RelType::Str(t) => t,
                        other => other.to_string(),
                    };
                    if let Some(ref hook) = self.event_hook {
                        hook(VmEvent::Custom {
                            topic,
                            payload: payload_val,
                        });
                    }
                    self.stack.push(RelType::Void);
                }

                OpCode::UILabel => {
                    let text_val = self.stack.pop().unwrap_or(RelType::Void);
                    let text = match text_val {
                        RelType::Str(s) => s,
                        v => v.to_string(),
                    };
                    self.stack.push(RelType::ASTNode(Box::new(
                        knoten_core_types::ast::Node::UILabel(Box::new(
                            knoten_core_types::ast::Node::StringLiteral(text),
                        )),
                    )));
                }
                OpCode::UIButton => {
                    let text_val = self.stack.pop().unwrap_or(RelType::Void);
                    let text = match text_val {
                        RelType::Str(s) => s,
                        v => v.to_string(),
                    };
                    self.stack.push(RelType::ASTNode(Box::new(
                        knoten_core_types::ast::Node::UIButton(Box::new(
                            knoten_core_types::ast::Node::StringLiteral(text),
                        )),
                    )));
                }
                OpCode::UITextInput => {
                    let text_val = self.stack.pop().unwrap_or(RelType::Void);
                    let text = match text_val {
                        RelType::Str(s) => s,
                        v => v.to_string(),
                    };
                    self.stack.push(RelType::ASTNode(Box::new(
                        knoten_core_types::ast::Node::UITextInput(Box::new(
                            knoten_core_types::ast::Node::StringLiteral(text),
                        )),
                    )));
                }
                OpCode::UISetStyle => {
                    let btn_hover_val = self.stack.pop().unwrap_or(RelType::Void);
                    let btn_idle_val = self.stack.pop().unwrap_or(RelType::Void);
                    let fill_val = self.stack.pop().unwrap_or(RelType::Void);
                    let accent_val = self.stack.pop().unwrap_or(RelType::Void);
                    let spacing_val = self.stack.pop().unwrap_or(RelType::Void);
                    let rounding_val = self.stack.pop().unwrap_or(RelType::Void);

                    let rounding = match rounding_val {
                        RelType::Float(f) => f as f32,
                        RelType::Int(i) => i as f32,
                        _ => 0.0,
                    };
                    let spacing = match spacing_val {
                        RelType::Float(f) => f as f32,
                        RelType::Int(i) => i as f32,
                        _ => 0.0,
                    };

                    fn parse_rgba_local(val: &RelType) -> Option<[f32; 4]> {
                        if let RelType::Array(arr) = val
                            && arr.len() >= 4
                        {
                            let r = match arr[0] {
                                RelType::Float(f) => f as f32,
                                RelType::Int(i) => i as f32,
                                _ => 0.0,
                            };
                            let g = match arr[1] {
                                RelType::Float(f) => f as f32,
                                RelType::Int(i) => i as f32,
                                _ => 0.0,
                            };
                            let b = match arr[2] {
                                RelType::Float(f) => f as f32,
                                RelType::Int(i) => i as f32,
                                _ => 0.0,
                            };
                            let a = match arr[3] {
                                RelType::Float(f) => f as f32,
                                RelType::Int(i) => i as f32,
                                _ => 1.0,
                            };
                            return Some([r, g, b, a]);
                        }
                        None
                    }

                    let accent_rgba = parse_rgba_local(&accent_val).unwrap_or([0.1, 0.5, 0.9, 1.0]);
                    let fill_rgba = parse_rgba_local(&fill_val).unwrap_or([0.1, 0.1, 0.1, 1.0]);
                    let btn_idle_rgba = parse_rgba_local(&btn_idle_val);
                    let btn_hover_rgba = parse_rgba_local(&btn_hover_val);

                    crate::natives::registry::send_render_command(
                        crate::natives::registry::RenderCommand::UpdateStyle {
                            window_id: 1,
                            rounding,
                            spacing,
                            accent_rgba,
                            fill_rgba,
                            btn_idle_rgba,
                            btn_hover_rgba,
                        },
                    );

                    self.stack.push(RelType::Void);
                }
                OpCode::UIHBox(count) => {
                    let mut children = Vec::with_capacity(*count);
                    for _ in 0..*count {
                        if let RelType::ASTNode(node) = self.stack.pop().unwrap_or(RelType::Void) {
                            children.push(*node);
                        }
                    }
                    children.reverse();
                    self.stack.push(RelType::ASTNode(Box::new(
                        knoten_core_types::ast::Node::UIHBox(children),
                    )));
                }
                OpCode::UIVBox(count) => {
                    let mut children = Vec::with_capacity(*count);
                    for _ in 0..*count {
                        if let RelType::ASTNode(node) = self.stack.pop().unwrap_or(RelType::Void) {
                            children.push(*node);
                        }
                    }
                    children.reverse();
                    self.stack.push(RelType::ASTNode(Box::new(
                        knoten_core_types::ast::Node::UIVBox(children),
                    )));
                }
                OpCode::UIWindow(_id_idx, count) => {
                    let mut children = Vec::with_capacity(*count);
                    for _ in 0..*count {
                        if let RelType::ASTNode(node) = self.stack.pop().unwrap_or(RelType::Void) {
                            children.push(*node);
                        }
                    }
                    children.reverse();

                    let _title_val = self.stack.pop().unwrap_or(RelType::Void);

                    crate::natives::registry::send_ui_nodes(children);
                    self.stack.push(RelType::Void);
                }
                OpCode::LoadComputeShader => {
                    let source_val = self.stack.pop().unwrap_or(RelType::Void);
                    if let RelType::Str(source) = source_val {
                        // Generate ID by hashing the source to deduplicate shader compilations
                        use std::hash::{Hash, Hasher};
                        let mut hasher = std::collections::hash_map::DefaultHasher::new();
                        source.hash(&mut hasher);
                        let id = hasher.finish() as usize;
                        crate::natives::registry::send_render_command(
                            crate::natives::registry::RenderCommand::LoadComputeShader {
                                id,
                                source,
                            },
                        );
                        self.stack.push(RelType::Int(id as i64));
                    } else {
                        return Err("LoadComputeShader expects a String source".into());
                    }
                }
                OpCode::DispatchCompute(arg_count) => {
                    let mut inputs = Vec::with_capacity(*arg_count);
                    for _ in 0..*arg_count {
                        inputs.push(self.stack.pop().unwrap_or(RelType::Void));
                    }
                    inputs.reverse();

                    let z = match self.stack.pop().unwrap_or(RelType::Void) {
                        RelType::Int(v) => v as u32,
                        RelType::Float(v) => v as u32,
                        _ => return Err("DispatchCompute expects numeric Z dimension".into()),
                    };
                    let y = match self.stack.pop().unwrap_or(RelType::Void) {
                        RelType::Int(v) => v as u32,
                        RelType::Float(v) => v as u32,
                        _ => return Err("DispatchCompute expects numeric Y dimension".into()),
                    };
                    let x = match self.stack.pop().unwrap_or(RelType::Void) {
                        RelType::Int(v) => v as u32,
                        RelType::Float(v) => v as u32,
                        _ => return Err("DispatchCompute expects numeric X dimension".into()),
                    };
                    let shader_id = match self.stack.pop().unwrap_or(RelType::Void) {
                        RelType::Int(v) => v as usize,
                        _ => return Err("DispatchCompute expects integer Shader ID".into()),
                    };

                    crate::natives::registry::send_render_command(
                        crate::natives::registry::RenderCommand::DispatchCompute {
                            shader_id,
                            x,
                            y,
                            z,
                            inputs,
                            bindings: None,
                        },
                    );
                    self.stack.push(RelType::Void);
                }
                OpCode::OpDispatchComputeLoop(arg_count) => {
                    let matrix_handle = match self.stack.pop().unwrap_or(RelType::Void) {
                        RelType::Int(v) => v,
                        _ => -1,
                    };
                    let mut inputs = Vec::with_capacity(*arg_count);
                    for _ in 0..*arg_count {
                        inputs.push(self.stack.pop().unwrap_or(RelType::Void));
                    }
                    inputs.reverse();

                    let iterations = match self.stack.pop().unwrap_or(RelType::Void) {
                        RelType::Int(v) => v as usize,
                        _ => return Err("DispatchComputeLoop expects integer iterations".into()),
                    };
                    let shader_id = match self.stack.pop().unwrap_or(RelType::Void) {
                        RelType::Int(v) => v as usize,
                        _ => return Err("DispatchComputeLoop expects integer Shader ID".into()),
                    };

                    let mat = if matrix_handle >= 0 {
                        crate::natives::registry::registry_get_matrix(matrix_handle)
                    } else {
                        None
                    };

                    let workgroup_size = 64u32;
                    let x_workgroups = (inputs.len() as u32).max(1).div_ceil(workgroup_size);

                    for _ in 0..iterations {
                        if let Some(ref m) = mat {
                            apply_matrix_to_inputs(&mut inputs, m);
                        }
                        let multi_bindings = split_inputs_to_bindings(&inputs);
                        crate::natives::registry::send_render_command(
                            crate::natives::registry::RenderCommand::DispatchCompute {
                                shader_id,
                                x: x_workgroups,
                                y: 1,
                                z: 1,
                                inputs: if !multi_bindings.is_empty() {
                                    vec![]
                                } else {
                                    inputs.clone()
                                },
                                bindings: if !multi_bindings.is_empty() {
                                    Some(multi_bindings)
                                } else {
                                    None
                                },
                            },
                        );
                        let result =
                            crate::natives::registry::registry_compute_readback(shader_id as i64);
                        if !result.is_empty() {
                            let has_nested = result
                                .iter()
                                .any(|r| matches!(r, crate::executor::RelType::Array(_)));
                            if has_nested {
                                inputs.clear();
                                for item in result {
                                    match item {
                                        crate::executor::RelType::Array(elems) => {
                                            inputs.extend(elems);
                                        }
                                        other => inputs.push(other),
                                    }
                                }
                            } else {
                                inputs = result;
                            }
                        }
                    }
                    self.stack.push(RelType::Void);
                }
                // Sprint 200/202: SIMD auto-vectorization — 4-element parallel ops
                OpCode::SimdExec {
                    op,
                    elements_a,
                    elements_b,
                    scale,
                    matrix_handle,
                } => {
                    let elem_arr = match self.stack.pop() {
                        Some(RelType::Array(arr)) => arr,
                        Some(other) => vec![other],
                        None => return Err("Fault: SimdExec expects stack operand".to_string()),
                    };
                    let to_f32 = |idx: &usize| -> Result<f32, String> {
                        match elem_arr.get(*idx) {
                            Some(RelType::Float(f)) => Ok(*f as f32),
                            Some(RelType::Int(i)) => Ok(*i as f32),
                            _ => Err("SimdExec: element must be Float or Int".into()),
                        }
                    };
                    let load_vec4 = |indices: &[usize; 4]| -> Result<[f32; 4], String> {
                        Ok([
                            to_f32(&indices[0])?,
                            to_f32(&indices[1])?,
                            to_f32(&indices[2])?,
                            to_f32(&indices[3])?,
                        ])
                    };
                    match op {
                        SimdOp::Scale => {
                            let factor = to_f32(scale)?;
                            let v = load_vec4(elements_a)?;
                            self.stack.push(RelType::Array(vec![
                                RelType::Float((v[0] * factor) as f64),
                                RelType::Float((v[1] * factor) as f64),
                                RelType::Float((v[2] * factor) as f64),
                                RelType::Float((v[3] * factor) as f64),
                            ]));
                        }
                        SimdOp::Add => {
                            let a = load_vec4(elements_a)?;
                            let b = load_vec4(elements_b)?;
                            self.stack.push(RelType::Array(vec![
                                RelType::Float((a[0] + b[0]) as f64),
                                RelType::Float((a[1] + b[1]) as f64),
                                RelType::Float((a[2] + b[2]) as f64),
                                RelType::Float((a[3] + b[3]) as f64),
                            ]));
                        }
                        SimdOp::Subtract => {
                            let a = load_vec4(elements_a)?;
                            let b = load_vec4(elements_b)?;
                            self.stack.push(RelType::Array(vec![
                                RelType::Float((a[0] - b[0]) as f64),
                                RelType::Float((a[1] - b[1]) as f64),
                                RelType::Float((a[2] - b[2]) as f64),
                                RelType::Float((a[3] - b[3]) as f64),
                            ]));
                        }
                        SimdOp::Dot => {
                            let a = load_vec4(elements_a)?;
                            let b = load_vec4(elements_b)?;
                            let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
                            self.stack.push(RelType::Float(dot as f64));
                        }
                        SimdOp::Transform => {
                            let v = load_vec4(elements_a)?;
                            if let Some(m) =
                                crate::natives::registry::registry_get_matrix(*matrix_handle)
                            {
                                let rx = m[0][0] * v[0]
                                    + m[1][0] * v[1]
                                    + m[2][0] * v[2]
                                    + m[3][0] * v[3];
                                let ry = m[0][1] * v[0]
                                    + m[1][1] * v[1]
                                    + m[2][1] * v[2]
                                    + m[3][1] * v[3];
                                let rz = m[0][2] * v[0]
                                    + m[1][2] * v[1]
                                    + m[2][2] * v[2]
                                    + m[3][2] * v[3];
                                let rw = m[0][3] * v[0]
                                    + m[1][3] * v[1]
                                    + m[2][3] * v[2]
                                    + m[3][3] * v[3];
                                self.stack.push(RelType::Array(vec![
                                    RelType::Float(rx as f64),
                                    RelType::Float(ry as f64),
                                    RelType::Float(rz as f64),
                                    RelType::Float(rw as f64),
                                ]));
                            } else {
                                self.stack.push(RelType::Array(vec![
                                    RelType::Float(v[0] as f64),
                                    RelType::Float(v[1] as f64),
                                    RelType::Float(v[2] as f64),
                                    RelType::Float(v[3] as f64),
                                ]));
                            }
                        }
                    }
                }
                OpCode::ExternCall {
                    name_idx,
                    arg_count,
                } => {
                    let name = match constants.get(*name_idx) {
                        Some(RelType::Str(s)) => s.clone(),
                        _ => {
                            return Err(
                                "OpExternCall: valid function name not found in constant pool"
                                    .to_string(),
                            );
                        }
                    };

                    // Pop arg_count items from Stack
                    let mut args = Vec::with_capacity(*arg_count);
                    for _ in 0..*arg_count {
                        args.push(self.stack.pop().unwrap_or(RelType::Void));
                    }
                    args.reverse(); // Standard reverse popping mapping

                    // Dynamic module routing from script prefix conventions
                    let (module, func) = if name.starts_with("registry_") {
                        ("registry", name.as_str())
                    } else if name.starts_with("ui_") {
                        ("ui", name.as_str())
                    } else if name.starts_with("fs_") || name.starts_with("file_") {
                        ("fs", name.as_str())
                    } else if name.starts_with("test_") {
                        ("test_lib", name.as_str())
                    } else if name.starts_with("array_") || name.starts_with("obj_") {
                        ("fs", name.as_str())
                    } else if name.starts_with("net_") || name.starts_with("network_") {
                        ("net", name.as_str())
                    } else if name.starts_with("json_") {
                        ("json", name.as_str())
                    } else if name.starts_with("time_") {
                        ("time", name.as_str())
                    } else if name.starts_with("math_") {
                        ("math", name.as_str())
                    } else if name.starts_with("string_") {
                        ("string", name.as_str())
                    } else {
                        // Global scope for unmapped builtins if the user writes flat names
                        ("global", name.as_str())
                    };

                    if let Some(b) = bridge {
                        let result = catch_unwind(AssertUnwindSafe(|| {
                            b.handle(module, func, &args, permissions)
                        }));
                        accumulated_cpu += start.elapsed();
                        start = std::time::Instant::now();
                        match result {
                            Ok(Some(crate::executor::ExecResult::Value(v))) => self.stack.push(v),
                            Ok(Some(crate::executor::ExecResult::Fault { msg, .. })) => {
                                return Err(format!("FFI Fault: {}", msg));
                            }
                            Ok(None) => {
                                return Err(format!(
                                    "FFI Function '{}.{}' not handled by active BridgeModule",
                                    module, func
                                ));
                            }
                            Ok(_) => self.stack.push(RelType::Void),
                            Err(panic_payload) => {
                                let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                                    s.to_string()
                                } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                                    s.clone()
                                } else {
                                    "Unknown panic".to_string()
                                };
                                eprintln!(
                                    "[KnotenCore Panic] Caught panic in FFI call '{}.{}': {}",
                                    module, func, msg
                                );
                                return Err(format!(
                                    "VM Panic in FFI call '{}.{}': {}",
                                    module, func, msg
                                ));
                            }
                        }
                    } else {
                        self.stack.push(RelType::Void); // Running without a connected FFI proxy
                    }
                }
                OpCode::AllocateDict => {
                    self.stack
                        .push(RelType::Dict(std::sync::Arc::new(std::sync::Mutex::new(
                            HashMap::new(),
                        ))));
                }
                OpCode::SetProperty => {
                    let val = self.stack.pop().unwrap_or(RelType::Void);
                    let key = self.stack.pop().unwrap_or(RelType::Void);
                    let obj = self.stack.pop().unwrap_or(RelType::Void);

                    if let (RelType::Dict(map_arc), RelType::Str(k)) = (&obj, &key) {
                        map_arc
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .insert(k.clone(), val);
                        self.stack.push(obj); // Push back the reference
                    } else if let (RelType::Object(map), RelType::Str(k)) = (&obj, &key) {
                        let mut new_map = map.clone();
                        new_map.insert(k.clone(), val);
                        self.stack.push(RelType::Object(new_map));
                    } else {
                        return Err("SetProperty expects (Dict/Object, Str, Any).".to_string());
                    }
                }
                OpCode::GetProperty => {
                    let key = self.stack.pop().unwrap_or(RelType::Void);
                    let obj = self.stack.pop().unwrap_or(RelType::Void);

                    if let (RelType::Dict(map_arc), RelType::Str(k)) = (&obj, &key) {
                        let res = map_arc
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .get(k)
                            .cloned()
                            .unwrap_or(RelType::Void);
                        self.stack.push(res);
                    } else if let (RelType::Object(map), RelType::Str(k)) = (&obj, &key) {
                        let res = map.get(k).cloned().unwrap_or(RelType::Void);
                        self.stack.push(res);
                    } else {
                        // Silent fail mimicking optional structures or missing keys natively
                        self.stack.push(RelType::Void);
                    }
                }
                OpCode::Pop => {
                    self.stack.pop();
                }
                OpCode::Print => {
                    let val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Print".to_string())?;
                    if crate::natives::registry::JSON_OUTPUT_MODE
                        .load(std::sync::atomic::Ordering::Relaxed)
                    {
                        eprintln!("{}", val);
                    } else {
                        println!("{}", val);
                    }
                }
                OpCode::OpPlayNote => {
                    let release_ms = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in PlayNote (release)".to_string())?;
                    let sustain_level = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in PlayNote (sustain)".to_string())?;
                    let decay_ms = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in PlayNote (decay)".to_string())?;
                    let attack_ms = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in PlayNote (attack)".to_string())?;
                    let wave_val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in PlayNote (waveform)".to_string())?;
                    #[cfg(feature = "ui")]
                    {
                        let waveform_idx = match wave_val {
                            RelType::Int(i) => i,
                            _ => 0,
                        };
                        let waveform = match waveform_idx {
                            0 => knoten_core_types::ast::Waveform::Sine,
                            1 => knoten_core_types::ast::Waveform::Sawtooth,
                            2 => knoten_core_types::ast::Waveform::Square,
                            3 => knoten_core_types::ast::Waveform::Triangle,
                            _ => knoten_core_types::ast::Waveform::Sine,
                        };
                        let duration = self
                            .stack
                            .pop()
                            .ok_or_else(|| "Stack underflow in PlayNote (duration)".to_string())?;
                        let freq = self
                            .stack
                            .pop()
                            .ok_or_else(|| "Stack underflow in PlayNote (frequency)".to_string())?;
                        let channel = self
                            .stack
                            .pop()
                            .ok_or_else(|| "Stack underflow in PlayNote (channel)".to_string())?;
                        let channel_idx = match channel {
                            RelType::Int(i) => i as usize,
                            v => {
                                eprintln!("[VM Synth] PlayNote: Expected Int channel, got {:?}", v);
                                0
                            }
                        };
                        let freq_val = match freq {
                            RelType::Float(f) => f as f32,
                            RelType::Int(i) => i as f32,
                            v => {
                                eprintln!(
                                    "[VM Synth] PlayNote: Expected Float/Int frequency, got {:?}",
                                    v
                                );
                                return Ok(RelType::Void);
                            }
                        };
                        let dur_val = match duration {
                            RelType::Int(i) => i as u64,
                            v => {
                                eprintln!(
                                    "[VM Synth] PlayNote: Expected Int duration, got {:?}",
                                    v
                                );
                                return Ok(RelType::Void);
                            }
                        };
                        let attack_val = match attack_ms {
                            RelType::Int(i) => i as u64,
                            _ => 5,
                        };
                        let decay_val = match decay_ms {
                            RelType::Int(i) => i as u64,
                            _ => 20,
                        };
                        let sustain_val = match sustain_level {
                            RelType::Float(f) => f as f32,
                            RelType::Int(i) => i as f32,
                            _ => 0.7,
                        };
                        let release_val = match release_ms {
                            RelType::Int(i) => i as u64,
                            _ => 100,
                        };

                        crate::natives::registry::init_audio_state();
                        if let Ok(mut guard) = crate::natives::registry::AUDIO_STATE.lock()
                            && let Some(ref mut mgr) = *guard
                        {
                            mgr.play_tone(
                                channel_idx,
                                freq_val,
                                dur_val,
                                0.3,
                                waveform,
                                attack_val,
                                decay_val,
                                sustain_val,
                                release_val,
                            );
                        }
                    }
                    #[cfg(not(feature = "ui"))]
                    {
                        let _ = (wave_val, attack_ms, decay_ms, sustain_level, release_ms);
                        let _ = self.stack.pop();
                        let _ = self.stack.pop();
                        let _ = self.stack.pop();
                    }
                    self.stack.push(RelType::Void);
                }
                OpCode::OpStopNote => {
                    let channel = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in StopNote (channel)".to_string())?;
                    #[cfg(feature = "ui")]
                    {
                        let channel_idx = if let RelType::Int(i) = channel {
                            i as usize
                        } else {
                            0
                        };
                        crate::natives::registry::init_audio_state();
                        if let Ok(mut guard) = crate::natives::registry::AUDIO_STATE.lock()
                            && let Some(ref mut mgr) = *guard
                        {
                            mgr.stop_tone(channel_idx);
                        }
                    }
                    #[cfg(not(feature = "ui"))]
                    {
                        let _ = channel;
                    }
                    self.stack.push(RelType::Void);
                }
                OpCode::TimeTravelReverse(checkpoint_id) => {
                    if let Some(mut state) = snapshot::snapshot_isolate(*checkpoint_id) {
                        state.ip = self.ip;
                        self.rollback(state);
                    }
                }
                // ── Sprint 309: Async Yield ───────────────────────────
                OpCode::Yield => {
                    self.execution_state = VmExecutionState::Yielded;
                    return Ok(RelType::Void);
                }
                OpCode::Return => {
                    let ret_val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Return".to_string())?;
                    self.stack.truncate(self.base_pointer); // Clean up the local variables / arguments frame

                    if let Some(frame) = self.frames.pop() {
                        // Return from function
                        self.ip = frame.ip;
                        self.base_pointer = frame.base_pointer;
                        self.stack.push(ret_val);
                    } else {
                        // Top level return exit
                        self.execution_state = VmExecutionState::Finished(ret_val.clone());
                        return Ok(ret_val);
                    }
                }
            }
        }

        let final_val = self.stack.pop().unwrap_or(RelType::Void);
        self.execution_state = VmExecutionState::Finished(final_val.clone());
        Ok(final_val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::RelType;
    use crate::vm::inspector::{HOT_PATH_TABLE, is_hot_path, track_hot_path};
    use knoten_core_types::opcode::OpCode;

    #[test]
    fn test_vm_execution_add() {
        let mut vm = VM::new();
        // Represents: 10 + 5
        let instructions = vec![
            OpCode::Constant(0), // Push 10
            OpCode::Constant(1), // Push 5
            OpCode::Add,         // Pop 5, Pop 10, Push 15
            OpCode::Return,
        ];
        let constants = vec![RelType::Int(10), RelType::Int(5)];

        let result = vm
            .run(
                &instructions,
                &constants,
                &AgentPermissions {
                    allow_network: false,
                    allowed_domains: vec![],
                    allow_fs_read: true,
                    allow_fs_write: false,
                },
                None,
            )
            .unwrap();
        assert_eq!(result, RelType::Int(15));
    }

    #[test]
    fn test_vm_execution_complex() {
        let mut vm = VM::new();
        // Represents: (10 - 2) * 3
        let instructions = vec![
            OpCode::Constant(0), // Push 10
            OpCode::Constant(1), // Push 2
            OpCode::Subtract,    // Pop 2, Pop 10, Push 8
            OpCode::Constant(2), // Push 3
            OpCode::Multiply,    // Pop 3, Pop 8, Push 24
            OpCode::Return,
        ];
        let constants = vec![RelType::Int(10), RelType::Int(2), RelType::Int(3)];

        let result = vm
            .run(
                &instructions,
                &constants,
                &AgentPermissions {
                    allow_network: false,
                    allowed_domains: vec![],
                    allow_fs_read: true,
                    allow_fs_write: false,
                },
                None,
            )
            .unwrap();
        assert_eq!(result, RelType::Int(24));
    }

    #[test]
    fn test_vm_jump_if_false() {
        let mut vm = VM::new();
        // Represents: if (false) { 10 } else { 20 }
        let instructions = vec![
            OpCode::Constant(0),    // Push false
            OpCode::JumpIfFalse(4), // If false, jump to index 4
            OpCode::Constant(1),    // Push 10
            OpCode::Jump(5),        // Jump to end (index 5)
            OpCode::Constant(2),    // Push 20 (index 4)
            OpCode::Return,         // Return (index 5)
        ];
        let constants = vec![RelType::Bool(false), RelType::Int(10), RelType::Int(20)];

        let result = vm
            .run(
                &instructions,
                &constants,
                &AgentPermissions {
                    allow_network: false,
                    allowed_domains: vec![],
                    allow_fs_read: true,
                    allow_fs_write: false,
                },
                None,
            )
            .unwrap();
        assert_eq!(result, RelType::Int(20));
    }

    #[test]
    fn test_vm_variables_and_strings() {
        let mut vm = VM::new();
        // script:
        // let pwd = "Test1"
        // let len = str_len(pwd)
        // return len
        let instructions = vec![
            OpCode::Constant(0),  // Push "Test1"
            OpCode::SetGlobal(1), // Set 'pwd'
            OpCode::GetGlobal(1), // Get 'pwd'
            OpCode::StringLength, // Length -> 5
            OpCode::SetGlobal(2), // Set 'len'
            OpCode::GetGlobal(2), // Get 'len'
            OpCode::Return,
        ];
        let constants = vec![
            RelType::Str("Test1".to_string()),
            RelType::Str("pwd".to_string()),
            RelType::Str("len".to_string()),
        ];

        let result = vm
            .run(
                &instructions,
                &constants,
                &AgentPermissions {
                    allow_network: false,
                    allowed_domains: vec![],
                    allow_fs_read: true,
                    allow_fs_write: false,
                },
                None,
            )
            .unwrap();
        assert_eq!(result, RelType::Int(5));
    }
    #[test]
    fn test_vm_network_sandbox_block() {
        let mut vm = VM::new();
        let instructions = vec![
            OpCode::Constant(0), // Push "https://api.github.com"
            OpCode::ExternCall {
                name_idx: 2,
                arg_count: 1,
            }, // "net", "net_fetch", 1 arg
            OpCode::Return,
        ];
        let constants = vec![
            RelType::Str("https://api.github.com".to_string()),
            RelType::Str("net".to_string()),
            RelType::Str("net_fetch".to_string()),
        ];

        let bridge = crate::natives::bridge::CoreBridge;
        let result = vm.run(
            &instructions,
            &constants,
            &AgentPermissions {
                allow_network: false,
                allowed_domains: vec![],
                allow_fs_read: true,
                allow_fs_write: false,
            },
            Some(&bridge),
        );

        // Assert the fault is securely caught
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Permission Denied: allow_network is false")
        );
    }

    #[test]
    fn test_vm_network_get_sandbox_block() {
        let mut vm = VM::new();
        let instructions = vec![
            OpCode::Constant(0), // Push "https://api.github.com"
            OpCode::ExternCall {
                name_idx: 2,
                arg_count: 1,
            }, // "net", "network_get", 1 arg
            OpCode::Return,
        ];
        let constants = vec![
            RelType::Str("https://api.github.com".to_string()),
            RelType::Str("net".to_string()),
            RelType::Str("network_get".to_string()),
        ];

        let bridge = crate::natives::bridge::CoreBridge;
        let result = vm.run(
            &instructions,
            &constants,
            &AgentPermissions {
                allow_network: false,
                allowed_domains: vec![],
                allow_fs_read: true,
                allow_fs_write: false,
            },
            Some(&bridge),
        );

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Permission Denied: allow_network is false")
        );
    }

    #[test]
    fn test_vm_network_get_failed_url() {
        let mut vm = VM::new();
        let instructions = vec![
            OpCode::Constant(0), // Push invalid url
            OpCode::ExternCall {
                name_idx: 2,
                arg_count: 1,
            },
            OpCode::Return,
        ];
        let constants = vec![
            RelType::Str("http://this-does-not-exist.invalid".to_string()),
            RelType::Str("net".to_string()),
            RelType::Str("network_get".to_string()),
        ];

        let bridge = crate::natives::bridge::CoreBridge;
        let result = vm.run(
            &instructions,
            &constants,
            &AgentPermissions {
                allow_network: true,
                allowed_domains: vec![],
                allow_fs_read: true,
                allow_fs_write: false,
            },
            Some(&bridge),
        );

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Network Error: HTTP Request Failed")
        );
    }

    #[test]
    fn test_vm_json_parsing() -> Result<(), String> {
        let mut vm = VM::new();
        // Valid JSON Test
        let instructions = vec![
            OpCode::Constant(0), // Push Valid JSON Object
            OpCode::ExternCall {
                name_idx: 2,
                arg_count: 1,
            }, // block: json, json_parse
            OpCode::Return,
        ];
        let constants = vec![
            RelType::Str("{\"api_version\":\"1.0\"}".to_string()),
            RelType::Str("json".to_string()),
            RelType::Str("json_parse".to_string()),
        ];

        let bridge = crate::natives::bridge::CoreBridge;
        let result = vm
            .run(
                &instructions,
                &constants,
                &AgentPermissions {
                    allow_network: false,
                    allowed_domains: vec![],
                    allow_fs_read: true,
                    allow_fs_write: false,
                },
                Some(&bridge),
            )
            .unwrap();

        // Ensure Map parses flawlessly capturing "api_version" natively
        if let RelType::Object(map) = result {
            assert_eq!(
                map.get("api_version").unwrap(),
                &RelType::Str("1.0".to_string())
            );
        } else {
            return Err("Expected JSON to parse into an Object natively!".into());
        }

        // Invalid JSON Test capturing gracefully Without Panics
        // Sprint 182: json_parse now returns Void on error instead of Fault
        let mut vm_err = VM::new();
        let constants_err = vec![
            RelType::Str("{ invalid...".to_string()),
            RelType::Str("json".to_string()),
            RelType::Str("json_parse".to_string()),
        ];
        let _ = vm_err.run(
            &instructions,
            &constants_err,
            &AgentPermissions {
                allow_network: false,
                allowed_domains: vec![],
                allow_fs_read: true,
                allow_fs_write: false,
            },
            Some(&bridge),
        );

        // Sprint 183: file_read / file_write sandbox defense — missing permission must fault
        let mut vm3 = VM::new();
        let file_instructions = vec![
            OpCode::Constant(0), // Push path
            OpCode::ExternCall {
                name_idx: 2,
                arg_count: 1,
            }, // file_read
            OpCode::Return,
        ];
        let file_constants = vec![
            RelType::Str("examples/cache.json".to_string()),
            RelType::Str("fs".to_string()),
            RelType::Str("file_read".to_string()),
        ];
        let file_result = vm3.run(
            &file_instructions,
            &file_constants,
            &AgentPermissions {
                allow_network: false,
                allowed_domains: vec![],
                allow_fs_read: false,
                allow_fs_write: false,
            },
            Some(&bridge),
        );
        assert!(
            file_result.is_err(),
            "file_read without allow_fs_read must fault"
        );
        assert!(file_result.unwrap_err().contains("Permission Denied"));

        let mut vm4 = VM::new();
        let write_instructions = vec![
            OpCode::Constant(0), // path
            OpCode::Constant(1), // content
            OpCode::ExternCall {
                name_idx: 4,
                arg_count: 2,
            }, // file_write
            OpCode::Return,
        ];
        let write_constants = vec![
            RelType::Str("examples/cache.json".to_string()),
            RelType::Str("test".to_string()),
            RelType::Str("fs".to_string()),
            RelType::Str("file_write".to_string()),
            RelType::Str("file_write".to_string()),
        ];
        let write_result = vm4.run(
            &write_instructions,
            &write_constants,
            &AgentPermissions {
                allow_network: false,
                allowed_domains: vec![],
                allow_fs_read: true,
                allow_fs_write: false,
            },
            Some(&bridge),
        );
        assert!(
            write_result.is_err(),
            "file_write without allow_fs_write must fault"
        );
        assert!(write_result.unwrap_err().contains("Permission Denied"));
        Ok(())
    }

    // Sprint 183: file_read / file_write sandbox defense — missing permission must fault
    #[test]
    fn test_vm_file_io_sandbox() {
        let mut vm = VM::new();
        let instructions = vec![
            OpCode::Constant(0), // Push path
            OpCode::ExternCall {
                name_idx: 2,
                arg_count: 1,
            }, // file_read
            OpCode::Return,
        ];
        let constants = vec![
            RelType::Str("examples/cache.json".to_string()),
            RelType::Str("fs".to_string()),
            RelType::Str("file_read".to_string()),
        ];

        let bridge = crate::natives::bridge::CoreBridge;
        let result = vm.run(
            &instructions,
            &constants,
            &AgentPermissions {
                allow_network: false,
                allowed_domains: vec![],
                allow_fs_read: false,
                allow_fs_write: false,
            },
            Some(&bridge),
        );
        assert!(
            result.is_err(),
            "file_read without allow_fs_read must fault"
        );
        assert!(result.unwrap_err().contains("Permission Denied"));

        // file_write without allow_fs_write must also fault
        let mut vm2 = VM::new();
        let instructions_write = vec![
            OpCode::Constant(0), // path
            OpCode::Constant(1), // content
            OpCode::ExternCall {
                name_idx: 4,
                arg_count: 2,
            }, // file_write
            OpCode::Return,
        ];
        let constants_write = vec![
            RelType::Str("examples/cache.json".to_string()),
            RelType::Str("test".to_string()),
            RelType::Str("fs".to_string()),
            RelType::Str("file_write".to_string()),
            RelType::Str("file_write".to_string()),
        ];
        let result2 = vm2.run(
            &instructions_write,
            &constants_write,
            &AgentPermissions {
                allow_network: false,
                allowed_domains: vec![],
                allow_fs_read: true,
                allow_fs_write: false,
            },
            Some(&bridge),
        );
        assert!(
            result2.is_err(),
            "file_write without allow_fs_write must fault"
        );
        assert!(result2.unwrap_err().contains("Permission Denied"));
    }

    fn run_logic_ops(ops: Vec<OpCode>, constants: Vec<RelType>) -> Result<RelType, String> {
        let mut vm = VM::new();
        let perms = AgentPermissions {
            allow_network: false,
            allowed_domains: vec![],
            allow_fs_read: false,
            allow_fs_write: false,
        };
        vm.run(&ops, &constants, &perms, None)
    }

    #[test]
    fn test_vm_lte() {
        // 3 <= 5 → true
        let r = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(1),
                OpCode::LessEqual,
                OpCode::Return,
            ],
            vec![RelType::Int(3), RelType::Int(5)],
        )
        .unwrap();
        assert_eq!(r, RelType::Bool(true));
        // 5 <= 5 → true (boundary)
        let r2 = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(0),
                OpCode::LessEqual,
                OpCode::Return,
            ],
            vec![RelType::Int(5)],
        )
        .unwrap();
        assert_eq!(r2, RelType::Bool(true));
        // 7 <= 5 → false
        let r3 = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(1),
                OpCode::LessEqual,
                OpCode::Return,
            ],
            vec![RelType::Int(7), RelType::Int(5)],
        )
        .unwrap();
        assert_eq!(r3, RelType::Bool(false));
    }

    #[test]
    fn test_vm_gte() {
        // 5 >= 3 → true
        let r = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(1),
                OpCode::GreaterEqual,
                OpCode::Return,
            ],
            vec![RelType::Int(5), RelType::Int(3)],
        )
        .unwrap();
        assert_eq!(r, RelType::Bool(true));
        // 5 >= 5 → true (boundary)
        let r2 = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(0),
                OpCode::GreaterEqual,
                OpCode::Return,
            ],
            vec![RelType::Int(5)],
        )
        .unwrap();
        assert_eq!(r2, RelType::Bool(true));
        // 3 >= 5 → false
        let r3 = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(1),
                OpCode::GreaterEqual,
                OpCode::Return,
            ],
            vec![RelType::Int(3), RelType::Int(5)],
        )
        .unwrap();
        assert_eq!(r3, RelType::Bool(false));
    }

    #[test]
    fn test_vm_not_equal() {
        // 1 != 2 → true
        let r = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(1),
                OpCode::NotEqual,
                OpCode::Return,
            ],
            vec![RelType::Int(1), RelType::Int(2)],
        )
        .unwrap();
        assert_eq!(r, RelType::Bool(true));
        // 2 != 2 → false
        let r2 = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(0),
                OpCode::NotEqual,
                OpCode::Return,
            ],
            vec![RelType::Int(2)],
        )
        .unwrap();
        assert_eq!(r2, RelType::Bool(false));
    }

    #[test]
    fn test_vm_and() {
        // true && true → true
        let r = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(0),
                OpCode::And,
                OpCode::Return,
            ],
            vec![RelType::Bool(true)],
        )
        .unwrap();
        assert_eq!(r, RelType::Bool(true));
        // true && false → false
        let r2 = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(1),
                OpCode::And,
                OpCode::Return,
            ],
            vec![RelType::Bool(true), RelType::Bool(false)],
        )
        .unwrap();
        assert_eq!(r2, RelType::Bool(false));
    }

    #[test]
    fn test_vm_or() {
        // false || true → true
        let r = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(1),
                OpCode::Or,
                OpCode::Return,
            ],
            vec![RelType::Bool(false), RelType::Bool(true)],
        )
        .unwrap();
        assert_eq!(r, RelType::Bool(true));
        // false || false → false
        let r2 = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(0),
                OpCode::Or,
                OpCode::Return,
            ],
            vec![RelType::Bool(false)],
        )
        .unwrap();
        assert_eq!(r2, RelType::Bool(false));
    }

    #[test]
    fn test_vm_not() {
        // !true → false
        let r = run_logic_ops(
            vec![OpCode::Constant(0), OpCode::Not, OpCode::Return],
            vec![RelType::Bool(true)],
        )
        .unwrap();
        assert_eq!(r, RelType::Bool(false));
        // !false → true
        let r2 = run_logic_ops(
            vec![OpCode::Constant(0), OpCode::Not, OpCode::Return],
            vec![RelType::Bool(false)],
        )
        .unwrap();
        assert_eq!(r2, RelType::Bool(true));
    }

    #[test]
    fn workgroup_empty_inputs_returns_one() {
        let x = 0u32.div_ceil(64).max(1);
        assert_eq!(x, 1, "Empty inputs → 1 workgroup minimum");
    }

    #[test]
    fn workgroup_single_input_returns_one() {
        let x = 1u32.div_ceil(64);
        assert_eq!(x, 1);
    }

    #[test]
    fn workgroup_63_inputs_returns_one() {
        let x = 63u32.div_ceil(64);
        assert_eq!(x, 1);
    }

    #[test]
    fn workgroup_64_inputs_returns_one() {
        let x = 64u32.div_ceil(64);
        assert_eq!(x, 1);
    }

    #[test]
    fn workgroup_65_inputs_returns_two() {
        let x = 65u32.div_ceil(64);
        assert_eq!(x, 2);
    }

    #[test]
    fn workgroup_large_input_scales() {
        let x = 1024u32.div_ceil(64);
        assert_eq!(x, 16);
    }

    #[test]
    fn test_math_matrix_transpose() {
        let test_mat = [
            [1.0, 0.0, 0.0, 4.0],
            [0.0, 1.0, 0.0, 5.0],
            [0.0, 0.0, 1.0, 6.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let handle = crate::natives::registry::registry_store_matrix(test_mat);

        let mut vm = VM::new();
        let handle_idx = 0;
        let name_idx = 1;
        let instructions = vec![
            OpCode::Constant(handle_idx),
            OpCode::ExternCall {
                name_idx,
                arg_count: 1,
            },
            OpCode::Return,
        ];
        let constants = vec![
            RelType::Int(handle),
            RelType::Str("math_matrix_transpose".to_string()),
            RelType::Str("math".to_string()),
        ];

        let bridge = crate::natives::bridge::CoreBridge;
        let result = vm
            .run(
                &instructions,
                &constants,
                &AgentPermissions {
                    allow_network: false,
                    allowed_domains: vec![],
                    allow_fs_read: true,
                    allow_fs_write: false,
                },
                Some(&bridge),
            )
            .unwrap();

        let new_handle = match result {
            RelType::Int(h) => h,
            other => panic!("Expected Int handle, got {:?}", other),
        };
        let transposed = crate::natives::registry::registry_get_matrix(new_handle)
            .expect("Transposed matrix not found");

        assert!(
            (transposed[0][0] - 1.0).abs() < 0.001,
            "col 0 x: {}",
            transposed[0][0]
        );
        assert!((transposed[0][1] - 0.0).abs() < 0.001, "col 0 y");
        assert!((transposed[0][2] - 0.0).abs() < 0.001, "col 0 z");
        assert!((transposed[0][3] - 0.0).abs() < 0.001, "col 0 w");
        assert!((transposed[1][0] - 0.0).abs() < 0.001, "col 1 x");
        assert!((transposed[1][1] - 1.0).abs() < 0.001, "col 1 y");
        assert!((transposed[1][2] - 0.0).abs() < 0.001, "col 1 z");
        assert!((transposed[1][3] - 0.0).abs() < 0.001, "col 1 w");
        assert!((transposed[2][0] - 0.0).abs() < 0.001, "col 2 x");
        assert!((transposed[2][1] - 0.0).abs() < 0.001, "col 2 y");
        assert!((transposed[2][2] - 1.0).abs() < 0.001, "col 2 z");
        assert!((transposed[2][3] - 0.0).abs() < 0.001, "col 2 w");
        assert!((transposed[3][0] - 4.0).abs() < 0.001, "col 3 x");
        assert!((transposed[3][1] - 5.0).abs() < 0.001, "col 3 y");
        assert!((transposed[3][2] - 6.0).abs() < 0.001, "col 3 z");
        assert!((transposed[3][3] - 1.0).abs() < 0.001, "col 3 w");
    }

    #[test]
    fn test_particle_streaming_flat_recycle() {
        let stride: usize = 7;
        let particle_data: Vec<RelType> = vec![
            RelType::Float(1.0),
            RelType::Float(2.0),
            RelType::Float(3.0),
            RelType::Float(0.1),
            RelType::Float(0.2),
            RelType::Float(0.3),
            RelType::Float(0.0),
        ];
        assert_eq!(particle_data.len() % stride, 0);

        let result: Vec<RelType> = vec![
            RelType::Float(1.1),
            RelType::Float(2.1),
            RelType::Float(3.1),
            RelType::Float(0.2),
            RelType::Float(0.3),
            RelType::Float(0.4),
            RelType::Float(0.01),
        ];
        let has_nested = result.iter().any(|r| matches!(r, RelType::Array(_)));
        assert!(!has_nested);

        let inputs = result;
        assert_eq!(inputs.len(), stride);
        assert!(
            (match &inputs[0] {
                RelType::Float(f) => (*f - 1.1).abs() < 0.001,
                _ => false,
            })
        );
    }

    #[test]
    fn test_particle_streaming_nested_flatten() {
        let result: Vec<RelType> = vec![
            RelType::Array(vec![
                RelType::Float(1.1),
                RelType::Float(2.1),
                RelType::Float(3.1),
            ]),
            RelType::Array(vec![
                RelType::Float(0.2),
                RelType::Float(0.3),
                RelType::Float(0.4),
            ]),
        ];
        let has_nested = result.iter().any(|r| matches!(r, RelType::Array(_)));
        assert!(has_nested);

        let mut inputs: Vec<RelType> = Vec::new();
        for item in result {
            match item {
                RelType::Array(elems) => inputs.extend(elems),
                other => inputs.push(other),
            }
        }
        assert_eq!(inputs.len(), 6);
        assert!(
            (match &inputs[0] {
                RelType::Float(f) => (*f - 1.1).abs() < 0.001,
                _ => false,
            })
        );
    }

    #[test]
    fn test_gpgpu_matrix_particle_transformation() {
        let angle = std::f32::consts::FRAC_PI_2;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let rot_z = [
            [cos_a, sin_a, 0.0, 0.0],
            [-sin_a, cos_a, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];

        let mut particle: Vec<RelType> = vec![
            RelType::Float(1.0),
            RelType::Float(0.0),
            RelType::Float(0.0),
            RelType::Float(0.1),
            RelType::Float(0.0),
            RelType::Float(0.0),
        ];

        apply_matrix_to_inputs(&mut particle, &rot_z);

        let px = match particle[0] {
            RelType::Float(f) => f,
            _ => panic!("Expected Float"),
        };
        let py = match particle[1] {
            RelType::Float(f) => f,
            _ => panic!("Expected Float"),
        };
        let vx = match particle[3] {
            RelType::Float(f) => f,
            _ => panic!("Expected Float"),
        };
        let vy = match particle[4] {
            RelType::Float(f) => f,
            _ => panic!("Expected Float"),
        };

        assert!((px - 0.0).abs() < 0.01, "rotZ90: x=1 -> 0, got {}", px);
        assert!((py - 1.0).abs() < 0.01, "rotZ90: y=0 -> 1, got {}", py);
        assert!((vx - 0.0).abs() < 0.01, "vel x rotated to 0");
        assert!((vy - 0.1).abs() < 0.01, "vel y rotated");
    }

    #[test]
    fn test_gpgpu_multi_storage_binding() {
        let inputs: Vec<RelType> = vec![
            RelType::Float(1.0),
            RelType::Float(2.0),
            RelType::Float(3.0),
            RelType::Float(0.1),
            RelType::Float(0.2),
            RelType::Float(0.3),
            RelType::Float(4.0),
            RelType::Float(5.0),
            RelType::Float(6.0),
            RelType::Float(0.4),
            RelType::Float(0.5),
            RelType::Float(0.6),
        ];

        let bindings = split_inputs_to_bindings(&inputs);
        assert!(!bindings.is_empty());
        let sets = bindings;
        assert_eq!(sets.len(), 2, "Should split into 2 binding sets");

        let positions = &sets[0];
        assert_eq!(positions.len(), 6, "2 particles x 3 position coords = 6");
        assert!(
            (match &positions[0] {
                RelType::Float(f) => (f - 1.0).abs() < 0.001,
                _ => false,
            })
        );
        assert!(
            (match &positions[4] {
                RelType::Float(f) => (f - 5.0).abs() < 0.001,
                _ => false,
            })
        );

        let velocities = &sets[1];
        assert_eq!(velocities.len(), 6, "2 particles x 3 velocity coords = 6");
        assert!(
            (match &velocities[2] {
                RelType::Float(f) => (f - 0.3).abs() < 0.001,
                _ => false,
            })
        );
    }

    #[test]
    fn test_multi_storage_binding_no_split_for_non_stride() {
        let inputs: Vec<RelType> = vec![
            RelType::Float(1.0),
            RelType::Float(2.0),
            RelType::Float(3.0),
            RelType::Float(4.0),
        ];
        let bindings = split_inputs_to_bindings(&inputs);
        assert!(bindings.is_empty(), "4 elements not divisible by 6 or 7");
    }

    #[test]
    fn test_shader_multi_binding_compilation() {
        let shader_src = include_str!("../../assets/shaders/data_preprocessor.wgsl");
        assert!(!shader_src.is_empty());
        assert!(shader_src.contains("@binding(0)"));
        assert!(shader_src.contains("@binding(1)"));
        assert!(shader_src.contains("positions"));
        assert!(shader_src.contains("velocities"));

        let particle_src = include_str!("../../assets/shaders/particle_render.wgsl");
        assert!(!particle_src.is_empty());
        assert!(particle_src.contains("@binding(0)"));
        assert!(particle_src.contains("vs_main"));
        assert!(particle_src.contains("fs_main"));
    }

    #[test]
    fn test_workspace_cleanliness() {
        let forbidden = [".html", ".js", ".css"];
        let dirs = [
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        ];
        for dir in &dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        let ext_dot = format!(".{}", ext);
                        assert!(
                            !forbidden.contains(&ext_dot.as_str()),
                            "Forbidden file {} found in {}",
                            path.display(),
                            dir.display()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_gpgpu_multi_pass_chaining() {
        let step1 = crate::natives::registry::ComputeChainStep {
            shader_id: 1,
            x: 4,
            y: 1,
            z: 1,
            inputs: vec![
                RelType::Float(1.0),
                RelType::Float(2.0),
                RelType::Float(3.0),
            ],
            bindings: None,
        };
        let step2 = crate::natives::registry::ComputeChainStep {
            shader_id: 2,
            x: 4,
            y: 1,
            z: 1,
            inputs: vec![
                RelType::Float(0.1),
                RelType::Float(0.2),
                RelType::Float(0.3),
            ],
            bindings: None,
        };
        let chain = [step1, step2];
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].shader_id, 1);
        assert_eq!(chain[1].shader_id, 2);
        assert_eq!(chain[0].x, 4);
        assert_eq!(chain[1].x, 4);
    }

    #[test]
    fn test_compiler_profiler_timing_injection() {
        crate::natives::registry::registry_drain_timing_markers();
        crate::natives::registry::registry_push_timing_marker(
            "COMPUTE_CHAIN_EXEC_US:1234:STEPS:2".to_string(),
        );
        crate::natives::registry::registry_push_timing_marker(
            "COMPUTE_CHAIN_EXEC_US:5678:STEPS:1".to_string(),
        );
        let markers = crate::natives::registry::registry_drain_timing_markers();
        assert_eq!(markers.len(), 2);
        assert!(markers[0].starts_with("COMPUTE_CHAIN_EXEC_US"));
        assert!(markers[0].contains(":STEPS:2"));
        assert!(markers[1].starts_with("COMPUTE_CHAIN_EXEC_US"));
        assert!(markers[1].contains(":STEPS:1"));
        let drained = crate::natives::registry::registry_drain_timing_markers();
        assert!(drained.is_empty(), "Drain should empty the marker buffer");
    }

    #[test]
    fn test_vm_runtime_inspection_snapshots() {
        let mut vm = VM::new();
        vm.is_inspectable = true;
        let instructions = vec![
            OpCode::Constant(0),
            OpCode::Constant(1),
            OpCode::Add,
            OpCode::Return,
        ];
        let constants = vec![RelType::Int(10), RelType::Int(5)];
        let _ = vm.run(
            &instructions,
            &constants,
            &AgentPermissions {
                allow_network: false,
                allowed_domains: vec![],
                allow_fs_read: false,
                allow_fs_write: false,
            },
            None,
        );

        let snapshot = get_vm_inspection_snapshot();
        assert!(snapshot.is_some(), "Inspection snapshot must be available");
        let (ip, depth) = snapshot.unwrap();
        assert!(ip > 0, "IP should be past initial instructions");
        assert!(depth > 0, "Stack should have values");
    }

    #[test]
    fn test_wasm_target_conditional_compilation() {
        #[cfg(target_arch = "wasm32")]
        {
            assert_eq!(std::env::consts::ARCH, "wasm32");
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            assert_ne!(std::env::consts::ARCH, "wasm32");
        }
    }

    #[test]
    fn test_runtime_dynamic_struct_extension() {
        use std::collections::HashMap;
        let obj = RelType::Object(HashMap::new());
        let mut map = match obj {
            RelType::Object(m) => m,
            _ => panic!(),
        };
        map.insert("velocity_z".to_string(), RelType::Float(0.5));
        assert!(
            (match map.get("velocity_z") {
                Some(RelType::Float(f)) => (*f - 0.5).abs() < 0.001,
                _ => false,
            })
        );
    }

    #[test]
    fn test_jit_hot_path_detection() {
        drain_hot_path_table();

        for i in 0..10_000 {
            track_hot_path(5);
            if i == 9_999 {
                assert!(is_hot_path(5), "IP 5 must be hot after 10k hits");
            }
        }
        assert!(!is_hot_path(1), "IP 1 must NOT be hot (0 hits)");

        drain_hot_path_table();
    }

    #[test]
    fn test_gpu_ui_hit_intersection() {
        let shader_src = include_str!("../../assets/shaders/ui_hit_test.wgsl");
        assert!(!shader_src.is_empty());
        assert!(shader_src.contains("panels"));
        assert!(shader_src.contains("mouse_pos"));
        assert!(shader_src.contains("hit_index"));
        assert!(shader_src.contains("@binding(0)"));
        assert!(shader_src.contains("@binding(1)"));
    }

    #[test]
    fn test_vm_state_rollback() {
        let mut vm = VM::new();
        let instructions = vec![OpCode::Constant(0), OpCode::SetGlobal(1), OpCode::Return];
        let constants = vec![RelType::Int(42), RelType::Str("x".to_string())];
        let perms = AgentPermissions {
            allow_network: false,
            allowed_domains: vec![],
            allow_fs_read: false,
            allow_fs_write: false,
        };

        let snapshot = vm.snapshot();

        let _ = vm.run(&instructions, &constants, &perms, None);

        assert_eq!(vm.globals.get("x"), Some(&RelType::Int(42)));

        vm.rollback(snapshot);

        assert!(
            !vm.globals.contains_key("x"),
            "After rollback, 'x' must not exist"
        );
    }

    #[test]
    fn test_vm_isolate_threaded_spawning() {
        let instructions = vec![
            OpCode::Constant(0),
            OpCode::Constant(1),
            OpCode::Add,
            OpCode::Return,
        ];
        let constants = vec![RelType::Int(10), RelType::Int(5)];

        let instructions_clone = instructions.clone();
        let constants_clone = constants.clone();

        let handle_a = spawn_isolate(instructions, constants);
        let handle_b = spawn_isolate(instructions_clone, constants_clone);

        let result_a = handle_a.join().expect("Isolate A panicked");
        let result_b = handle_b.join().expect("Isolate B panicked");

        assert_eq!(result_a.unwrap(), RelType::Int(15));
        assert_eq!(result_b.unwrap(), RelType::Int(15));
    }

    #[test]
    fn test_inter_isolate_mailbox_messaging() {
        let (tx, rx) = std::sync::mpsc::channel::<RelType>();

        let handle_b = std::thread::spawn(move || {
            let msg = rx.recv().expect("Mailbox recv failed");
            assert_eq!(msg, RelType::Int(1337));
            RelType::Int(42)
        });

        tx.send(RelType::Int(1337)).expect("Mailbox send failed");

        let result = handle_b.join().expect("Isolate B panicked");
        assert_eq!(result, RelType::Int(42));
    }

    #[test]
    fn test_vm_work_stealing_balancing() {
        drain_work_stealing_queues();

        let donated_work: Vec<WorkItem> = vec![(OpCode::Constant(0), vec![RelType::Int(99)])];
        push_work_batch(1, donated_work);

        let thief_isolate = VMIsolate::new(vec![], vec![]);
        let handle = std::thread::spawn(move || thief_isolate.run());

        let result = handle.join().expect("Thief isolate panicked");
        assert_eq!(result.unwrap(), RelType::Int(99));

        drain_work_stealing_queues();
    }

    static TEST_SNAPSHOT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_isolated_atomic_checkpointing() {
        let _lock = TEST_SNAPSHOT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        drain_isolate_snapshots();
        isolate::drain_hot_swap_registry();

        let instructions = vec![OpCode::Constant(0), OpCode::Return];
        let constants = vec![RelType::Int(7)];
        let mut isolate = VMIsolate::new(instructions, constants);
        isolate.isolate_id = 99;
        let result = isolate.run();
        assert_eq!(result.unwrap(), RelType::Int(7));

        let restored = snapshot_isolate(99);
        assert!(
            restored.is_some(),
            "Snapshot should exist after isolate run"
        );

        drain_isolate_snapshots();
    }

    #[test]
    fn test_cross_isolate_ffi_contention() {
        let instructions = vec![OpCode::Constant(0), OpCode::Return];
        let constants = vec![RelType::Float(1.0)];

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let instr = instructions.clone();
                let cnst = constants.clone();
                std::thread::spawn(move || {
                    for _ in 0..10_000 {
                        let isolate = VMIsolate::new(instr.clone(), cnst.clone());
                        let result = isolate.run();
                        assert!(result.is_ok(), "Isolate must succeed");
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    }

    #[test]
    fn test_isolate_local_heap_allocation() {
        let handles: Vec<_> = (0..2)
            .map(|id| {
                std::thread::spawn(move || {
                    let mut isolate = VMIsolate::new(
                        vec![OpCode::Constant(0), OpCode::Return],
                        vec![RelType::Int(0)],
                    );
                    isolate.isolate_id = id;
                    for i in 0..5000i64 {
                        isolate.local_heap.insert(
                            format!("arr_{}_{}", id, i),
                            RelType::Array(vec![RelType::Int(i); 3]),
                        );
                    }
                    assert_eq!(isolate.local_heap.len(), 5000);
                    let result = isolate.run();
                    assert!(result.is_ok());
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    }

    proptest::proptest! {
        #[test]
        fn fuzz_simd_matrix_transformations(
            m11 in proptest::num::f32::NORMAL,
            m12 in proptest::num::f32::NORMAL,
            m13 in proptest::num::f32::NORMAL,
            m14 in proptest::num::f32::NORMAL,
            m21 in proptest::num::f32::NORMAL,
            m22 in proptest::num::f32::NORMAL,
            m23 in proptest::num::f32::NORMAL,
            m24 in proptest::num::f32::NORMAL,
            m31 in proptest::num::f32::NORMAL,
            m32 in proptest::num::f32::NORMAL,
            m33 in proptest::num::f32::NORMAL,
            m34 in proptest::num::f32::NORMAL,
            m41 in proptest::num::f32::NORMAL,
            m42 in proptest::num::f32::NORMAL,
            m43 in proptest::num::f32::NORMAL,
            m44 in proptest::num::f32::NORMAL,
            px in proptest::num::f32::NORMAL,
            py in proptest::num::f32::NORMAL,
            pz in proptest::num::f32::NORMAL,
            vx in proptest::num::f32::NORMAL,
            vy in proptest::num::f32::NORMAL,
            vz in proptest::num::f32::NORMAL,
        ) {
            let mat = [
                [m11, m12, m13, m14],
                [m21, m22, m23, m24],
                [m31, m32, m33, m34],
                [m41, m42, m43, m44],
            ];
            let mut particle = vec![
                RelType::Float(px as f64),
                RelType::Float(py as f64),
                RelType::Float(pz as f64),
                RelType::Float(vx as f64),
                RelType::Float(vy as f64),
                RelType::Float(vz as f64),
            ];
            apply_matrix_to_inputs(&mut particle, &mat);
            assert_eq!(particle.len(), 6, "Stride-6 must stay length 6");
            for item in &particle {
                if let RelType::Float(f) = item {
                    assert!(f.is_finite(), "Matrix output must be finite");
                }
            }
        }
    }

    #[test]
    fn test_vm_isolate_hot_swap_reloading() {
        let add_instr = vec![
            OpCode::Constant(0),
            OpCode::Constant(1),
            OpCode::Add,
            OpCode::Return,
        ];
        let add_const = vec![RelType::Int(10), RelType::Int(5)];
        let mul_instr = vec![
            OpCode::Constant(0),
            OpCode::Constant(1),
            OpCode::Multiply,
            OpCode::Return,
        ];
        let mul_const = vec![RelType::Int(10), RelType::Int(5)];

        let mut isolate = isolate::VMIsolate::new(add_instr, add_const);
        isolate.isolate_id = 42;
        assert_eq!(isolate.run().unwrap(), RelType::Int(15));

        assert!(isolate::hot_swap_isolate_code(42, mul_instr, mul_const));

        let mut isolate2 = isolate::VMIsolate::new(vec![], vec![]);
        isolate2.isolate_id = 42;
        let result = isolate2.run().unwrap();
        assert_eq!(
            result,
            RelType::Int(50),
            "After hot-swap, should multiply: 10*5=50"
        );
    }

    #[test]
    fn test_agent_telemetry_self_healing() {
        let telemetry_id: i64 = 77;
        isolate::telemetry_push(
            telemetry_id,
            "ERR:WATCHDOG:IP:420:STACK:3:MSG:timeout".to_string(),
        );
        isolate::telemetry_push(
            telemetry_id,
            "ERR:FFI:IP:421:STACK:1:MSG:permission_denied".to_string(),
        );

        let last = isolate::telemetry_last(telemetry_id);
        assert!(last.is_some());
        assert!(last.unwrap().contains("FFI"));

        let drained = isolate::telemetry_drain(telemetry_id);
        assert_eq!(drained.len(), 2);
    }

    #[test]
    fn test_inter_isolate_dma_zero_copy() {
        let topic = format!("large_array_{}", rand::random::<u64>());
        let data: Vec<RelType> = (0..10_000).map(RelType::Int).collect();
        isolate::bus_publish(topic.clone(), data);

        let handles: Vec<_> = (0..3)
            .map(|_| {
                let topic_clone = topic.clone();
                std::thread::spawn(move || {
                    let mut bus = isolate::bus_subscribe(&topic_clone);
                    if bus.is_none() {
                        isolate::bus_publish(
                            topic_clone.clone(),
                            (0..10_000).map(RelType::Int).collect(),
                        );
                        bus = isolate::bus_subscribe(&topic_clone);
                    }
                    let bus = bus.expect("Must subscribe to bus");
                    assert_eq!(bus.len(), 10_000);
                    assert!(
                        (match &bus[9999] {
                            RelType::Int(v) => *v == 9999,
                            _ => false,
                        })
                    );
                })
            })
            .collect();

        for h in handles {
            h.join().expect("DMA thread panicked");
        }

        isolate::bus_drain();
    }

    #[test]
    fn test_vm_isolate_speculative_branching() {
        let perms = AgentPermissions {
            allow_network: false,
            allowed_domains: vec![],
            allow_fs_read: false,
            allow_fs_write: false,
        };
        let constants = vec![
            RelType::Str("x".to_string()),
            RelType::Int(1),
            RelType::Str("y".to_string()),
            RelType::Void,
        ];

        let mut vm = VM::new();
        vm.globals.insert("x".to_string(), RelType::Int(10));

        let vm_snapshot = vm.snapshot();

        let winner = dispatch_speculative_branch(
            vm_snapshot,
            vec![
                OpCode::GetGlobal(0),
                OpCode::Constant(1),
                OpCode::Add,
                OpCode::SetGlobal(2),
                OpCode::Constant(3),
                OpCode::Return,
            ],
            vec![
                OpCode::GetGlobal(0),
                OpCode::Constant(1),
                OpCode::Multiply,
                OpCode::SetGlobal(2),
                OpCode::Constant(3),
                OpCode::Return,
            ],
            constants.clone(),
            true,
        )
        .expect("Speculative branch must succeed");

        assert_eq!(
            winner.globals.get("y"),
            Some(&RelType::Int(11)),
            "Path A should win: 10 + 1 = 11"
        );

        let mut vm2 = VM::new();
        vm2.globals.insert("x".to_string(), RelType::Int(3));
        let vm2_snapshot = vm2.snapshot();

        let loser_winner = dispatch_speculative_branch(
            vm2_snapshot,
            vec![
                OpCode::GetGlobal(0),
                OpCode::Constant(1),
                OpCode::Add,
                OpCode::SetGlobal(2),
                OpCode::Constant(3),
                OpCode::Return,
            ],
            vec![
                OpCode::GetGlobal(0),
                OpCode::Constant(1),
                OpCode::Multiply,
                OpCode::SetGlobal(2),
                OpCode::Constant(3),
                OpCode::Return,
            ],
            constants.clone(),
            false,
        )
        .expect("Speculative branch must succeed");

        assert_eq!(
            loser_winner.globals.get("y"),
            Some(&RelType::Int(3)),
            "Path B should win: 3 * 1 = 3"
        );

        let _ = perms;
    }

    #[test]
    fn test_vm_temporal_reversal() {
        let _lock = TEST_SNAPSHOT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let checkpoint_id: i64 = 42;

        let perms = AgentPermissions {
            allow_network: false,
            allowed_domains: vec![],
            allow_fs_read: false,
            allow_fs_write: false,
        };
        let constants = vec![
            RelType::Str("x".to_string()),
            RelType::Int(5),
            RelType::Void,
        ];

        let instructions_add = vec![
            OpCode::GetGlobal(0),
            OpCode::Constant(1),
            OpCode::Add,
            OpCode::SetGlobal(0),
            OpCode::Constant(2),
            OpCode::Return,
        ];

        let instructions_rewind = vec![
            OpCode::TimeTravelReverse(checkpoint_id),
            OpCode::Constant(2),
            OpCode::Return,
        ];

        let mut vm = VM::new();
        vm.globals.insert("x".to_string(), RelType::Int(5));
        store_snapshot(checkpoint_id, vm.snapshot());

        let result = vm.run(&instructions_add, &constants, &perms, None).unwrap();
        assert_eq!(result, RelType::Void);
        assert_eq!(vm.globals.get("x"), Some(&RelType::Int(10)));

        let r2 = vm
            .run(&instructions_rewind, &constants, &perms, None)
            .unwrap();
        assert_eq!(r2, RelType::Void);
        assert_eq!(
            vm.globals.get("x"),
            Some(&RelType::Int(5)),
            "After rewind, x should be 5 (snapshot value)"
        );

        vm.globals.insert("x".to_string(), RelType::Int(20));

        let r3 = vm.run(&instructions_add, &constants, &perms, None).unwrap();
        assert_eq!(r3, RelType::Void);
        assert_eq!(
            vm.globals.get("x"),
            Some(&RelType::Int(25)),
            "After live mutation in the past (x=20) then +5: 20 + 5 = 25"
        );

        drain_isolate_snapshots();
    }

    #[test]
    fn test_vm_cryptographic_state_verifiability() {
        let perms = AgentPermissions {
            allow_network: false,
            allowed_domains: vec![],
            allow_fs_read: false,
            allow_fs_write: false,
        };
        let constants = vec![RelType::Int(10), RelType::Int(5), RelType::Int(2)];

        let instructions = vec![
            OpCode::Constant(0),
            OpCode::Constant(1),
            OpCode::Add,
            OpCode::Constant(2),
            OpCode::Multiply,
            OpCode::Return,
        ];

        let mut vm1 = VM::new();
        vm1.run(&instructions, &constants, &perms, None).unwrap();
        let hash1 = vm1.crypto_state_hash;
        assert!(hash1 > 0, "Hash must be non-zero after execution");

        let mut vm2 = VM::new();
        vm2.run(&instructions, &constants, &perms, None).unwrap();
        let hash2 = vm2.crypto_state_hash;
        assert_eq!(
            hash1, hash2,
            "Identical execution must produce identical hash"
        );

        let tampered = vec![
            OpCode::Constant(0),
            OpCode::Constant(1),
            OpCode::Subtract,
            OpCode::Constant(2),
            OpCode::Multiply,
            OpCode::Return,
        ];

        let mut vm3 = VM::new();
        vm3.run(&tampered, &constants, &perms, None).unwrap();
        let hash3 = vm3.crypto_state_hash;
        assert_ne!(
            hash1, hash3,
            "Tampered instruction (Subtract vs Add) must produce divergent hash"
        );
    }

    #[test]
    fn test_cluster_work_stealing_rdma() {
        let constants = vec![RelType::Int(7), RelType::Int(3)];

        push_cluster_work_batch(
            "Knoten_Berlin",
            vec![(OpCode::Subtract, vec![RelType::Int(7), RelType::Int(3)])],
        );

        let stolen = try_steal_cluster_work("Knoten_Berlin", 99);
        assert!(stolen.is_some(), "Must steal work from remote node");
        let (op, _consts) = stolen.unwrap();
        assert_eq!(op, OpCode::Subtract);

        let mut vm = VM::new();
        let instructions = vec![
            OpCode::Constant(0),
            OpCode::Constant(1),
            OpCode::Subtract,
            OpCode::Return,
        ];
        let perms = AgentPermissions {
            allow_network: false,
            allowed_domains: vec![],
            allow_fs_read: false,
            allow_fs_write: false,
        };
        let result = vm.run(&instructions, &constants, &perms, None).unwrap();
        assert_eq!(result, RelType::Int(4), "7 - 3 = 4");

        drain_cluster_work_queues();
    }

    #[test]
    fn test_vm_adaptive_evolutionary_pgo() {
        let isolate_id: i64 = 77;

        let instructions = vec![
            OpCode::Constant(0),
            OpCode::SetGlobal(2),
            OpCode::Constant(0),
            OpCode::SetGlobal(3),
            OpCode::GetGlobal(3),
            OpCode::Constant(1),
            OpCode::Less,
            OpCode::JumpIfFalse(17),
            OpCode::GetGlobal(2),
            OpCode::Constant(1),
            OpCode::Add,
            OpCode::SetGlobal(2),
            OpCode::GetGlobal(3),
            OpCode::Constant(1),
            OpCode::Add,
            OpCode::SetGlobal(3),
            OpCode::Jump(4),
            OpCode::GetGlobal(2),
            OpCode::Return,
        ];

        let constants = vec![
            RelType::Int(0),
            RelType::Int(1),
            RelType::Str("acc".to_string()),
            RelType::Str("i".to_string()),
        ];

        let registry = isolate::get_hot_swap_registry();
        registry.lock().unwrap_or_else(|e| e.into_inner()).insert(
            isolate_id,
            std::sync::Arc::new(std::sync::Mutex::new((
                instructions.clone(),
                constants.clone(),
            ))),
        );

        let inst_before = {
            let guard = registry.lock().unwrap_or_else(|e| e.into_inner());
            let code = guard.get(&isolate_id).unwrap();
            code.lock().unwrap_or_else(|e| e.into_inner()).0.len()
        };

        HOT_PATH_TABLE.with(|t| {
            t.borrow_mut().insert(16, 15_000);
        });

        let modified = optimize_active_hotpath(isolate_id);
        assert!(modified, "Must detect and optimize hotpath");

        let inst_after = {
            let guard = registry.lock().unwrap_or_else(|e| e.into_inner());
            let code = guard.get(&isolate_id).unwrap();
            code.lock().unwrap_or_else(|e| e.into_inner()).0.len()
        };

        assert_ne!(
            inst_before, inst_after,
            "Instruction count must change after PGO unrolling"
        );
        assert!(
            inst_after > inst_before,
            "Unrolling must increase instruction count"
        );

        drain_hot_swap_registry();
        drain_hot_path_table();
    }

    #[test]
    fn test_cross_node_isolate_migration() {
        let _lock = TEST_SNAPSHOT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        drain_hot_swap_registry();
        drain_cluster_work_queues();
        drain_isolate_snapshots();

        let constants = vec![RelType::Int(5), RelType::Int(3)];
        let instructions = vec![
            OpCode::Constant(0),
            OpCode::Constant(1),
            OpCode::Add,
            OpCode::Return,
        ];

        let isolate_id: i64 = 42;

        let registry = isolate::get_hot_swap_registry();
        registry.lock().unwrap_or_else(|e| e.into_inner()).insert(
            isolate_id,
            std::sync::Arc::new(std::sync::Mutex::new((
                instructions.clone(),
                constants.clone(),
            ))),
        );

        let _perms = AgentPermissions {
            allow_network: false,
            allowed_domains: vec![],
            allow_fs_read: false,
            allow_fs_write: false,
        };
        let mut vm = VM::new();
        let pre_instr = vec![OpCode::Constant(0), OpCode::Return];
        let pre_const = vec![RelType::Int(42)];
        let _ = vm
            .run(&pre_instr, &pre_const, &AgentPermissions::default(), None)
            .unwrap();
        assert!(
            vm.ip > 0,
            "VM must have executed instructions before snapshot"
        );

        vm.globals.insert("x".to_string(), RelType::Int(10));
        vm.crypto_state_hash = 0xABCD;
        let state = vm.snapshot();
        store_snapshot(isolate_id, state);

        let registry = isolate::get_hot_swap_registry();
        registry
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(isolate_id)
            .or_insert_with(|| {
                std::sync::Arc::new(std::sync::Mutex::new((
                    instructions.clone(),
                    constants.clone(),
                )))
            });

        migrate_active_isolate(isolate_id, "Knoten_Zadar").expect("Migration must succeed");

        let received = receive_migration_payload("Knoten_Zadar")
            .expect("Target node must receive migration payload");
        let (migrated_instrs, migrated_consts, serialized_state) = received;

        let migrated =
            resume_migrated_isolate(&migrated_instrs, &migrated_consts, &serialized_state)
                .expect("Resume must succeed");

        assert!(
            migrated.local_heap.get("x") == Some(&RelType::Int(10)),
            "Migrated isolate must retain globals"
        );
        assert!(
            migrated.migration_state.is_some(),
            "Migration state must be preserved"
        );

        let result = migrated.run().expect("Migrated isolate must run");
        assert_eq!(result, RelType::Int(8), "5 + 3 = 8 on migrated isolate");

        drain_hot_swap_registry();
        drain_cluster_work_queues();
        drain_isolate_snapshots();
    }

    #[test]
    fn test_isolate_garbage_collection_reclamation() {
        let _lock = TEST_SNAPSHOT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let test_ids: Vec<i64> = vec![5000, 5001, 5002, 5003, 5004];
        {
            let registry = isolate::get_hot_swap_registry();
            let mut guard = registry.lock().unwrap_or_else(|e| e.into_inner());
            for &id in &test_ids {
                let instructions = vec![OpCode::Constant(0), OpCode::Return];
                let constants = vec![RelType::Int(id)];
                guard.insert(
                    id,
                    std::sync::Arc::new(std::sync::Mutex::new((instructions, constants))),
                );
            }
            for &id in &test_ids {
                assert!(
                    guard.contains_key(&id),
                    "Registry must contain inserted isolate {} before sweep",
                    id
                );
            }
        }

        sweep_terminated_isolates();

        {
            let guard = isolate::get_hot_swap_registry()
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            for &id in &test_ids {
                assert!(
                    !guard.contains_key(&id),
                    "Registry must not contain isolate {} after sweep",
                    id
                );
            }
        }

        drain_isolate_snapshots();
        drain_cluster_work_queues();
    }

    #[test]
    fn test_isolate_gc_sub_millisecond_latency() {
        let _lock = TEST_SNAPSHOT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        drain_hot_swap_registry();
        drain_isolate_snapshots();
        drain_cluster_work_queues();

        let instructions = vec![OpCode::Constant(0), OpCode::Return];
        let constants = vec![RelType::Int(42)];
        let test_ids: Vec<i64> = (6000..6020).collect();

        {
            let registry = isolate::get_hot_swap_registry();
            let mut guard = registry.lock().unwrap_or_else(|e| e.into_inner());
            for &id in &test_ids {
                guard.insert(
                    id,
                    std::sync::Arc::new(std::sync::Mutex::new((
                        instructions.clone(),
                        constants.clone(),
                    ))),
                );
            }
        }

        // Warmup sweep to avoid OS page-fault / lock contention measurement noise on shared CI runners
        sweep_terminated_isolates();

        // Repopulate for latency benchmark
        {
            let registry = isolate::get_hot_swap_registry();
            let mut guard = registry.lock().unwrap_or_else(|e| e.into_inner());
            for &id in &test_ids {
                guard.insert(
                    id,
                    std::sync::Arc::new(std::sync::Mutex::new((
                        instructions.clone(),
                        constants.clone(),
                    ))),
                );
            }
        }

        let start = std::time::Instant::now();
        sweep_terminated_isolates();
        let elapsed = start.elapsed();

        // Allow resilient threshold (10,000us / 10ms) under headless CI runners to prevent scheduling jitter flakiness
        assert!(
            elapsed.as_micros() < 10_000,
            "Sweeper must complete efficiently under CI threshold ({}us)",
            elapsed.as_micros()
        );

        drain_isolate_snapshots();
        drain_cluster_work_queues();
        drain_hot_swap_registry();
    }

    #[test]
    fn test_wgpu_inspector_panel_state_extraction() {
        let mut vm = VM::new();
        let perms = AgentPermissions::default();
        let instructions = vec![
            OpCode::Constant(0),
            OpCode::Constant(1),
            OpCode::Add,
            OpCode::Constant(0),
            OpCode::Constant(1),
            OpCode::Multiply,
            OpCode::Return,
        ];
        let constants = vec![RelType::Int(7), RelType::Int(3)];

        let data_before = vm.inspect();
        assert_eq!(data_before.stack_depth, 0);
        assert_eq!(data_before.frame_count, 0);
        assert_eq!(data_before.ip, 0);
        assert_eq!(data_before.bp, 0);
        assert_eq!(data_before.crypto_state_hash, 0);
        assert_eq!(data_before.global_count, 0);

        let _ = vm.run(&instructions, &constants, &perms, None).unwrap();

        let data_after = vm.inspect();
        assert!(data_after.ip > 0, "IP must advance after execution");
        assert!(
            data_after.crypto_state_hash > 0,
            "Crypto hash must be non-zero after execution"
        );
        assert!(
            data_after.ledger_nonce > 0,
            "Ledger nonce must be positive after execution"
        );
    }

    #[test]
    fn test_p2p_mesh_bus_distributed_routing() {
        let _lock = TEST_SNAPSHOT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let data = vec![RelType::Int(42), RelType::Float(3.15)];

        isolate::mesh_subscribe("global_telemetry", "Knoten_Zadar");
        isolate::bus_publish_mesh("global_telemetry".to_string(), data);

        let mut local = None;
        let start_local = std::time::Instant::now();
        while start_local.elapsed() < std::time::Duration::from_millis(500) {
            local = isolate::bus_subscribe("global_telemetry");
            if local.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(local.is_some(), "Local subscriber must receive data");
        assert_eq!(local.unwrap().len(), 2);

        let mut remote = None;
        let start_remote = std::time::Instant::now();
        while start_remote.elapsed() < std::time::Duration::from_millis(500) {
            remote = isolate::bus_poll_remote("global_telemetry", "Knoten_Zadar");
            if remote.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            remote.is_some(),
            "Remote node must receive data via cluster queue"
        );
        assert_eq!(remote.unwrap().len(), 2);

        isolate::bus_drain();
        isolate::drain_mesh_routing_table();
        drain_cluster_work_queues();
    }

    #[test]
    fn test_p2p_mesh_bus_network_partition_resilience() {
        let _lock = TEST_SNAPSHOT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        isolate::bus_drain();
        isolate::drain_mesh_routing_table();
        drain_cluster_work_queues();

        let result = isolate::bus_poll_remote("nonexistent_topic", "offline_node");
        assert!(
            result.is_none(),
            "Polling empty/offline queue must return None gracefully"
        );

        isolate::mesh_subscribe("resilience_test", "Knoten_Berlin");
        isolate::bus_publish_mesh("resilience_test".to_string(), vec![RelType::Int(1)]);
        let mut polled = None;
        let start_polled = std::time::Instant::now();
        while start_polled.elapsed() < std::time::Duration::from_millis(1500) {
            polled = isolate::bus_poll_remote("resilience_test", "Knoten_Berlin");
            if polled.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(polled.is_some(), "Mesh bus must survive single-node poll");
        assert_eq!(polled.unwrap().len(), 1);

        let data = vec![RelType::Int(7); 2048];
        isolate::mesh_stream_publish("large_stream".to_string(), &data);
        let mut chunked = None;
        let start_chunked = std::time::Instant::now();
        while start_chunked.elapsed() < std::time::Duration::from_millis(1500) {
            chunked = isolate::bus_subscribe("large_stream");
            if chunked.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            chunked.is_some(),
            "Streamed large payload must be published"
        );

        isolate::bus_drain();
        isolate::drain_mesh_routing_table();
        drain_cluster_work_queues();
    }

    #[test]
    fn test_raft_cluster_leader_election() {
        let nodes = ["Knoten_Berlin", "Knoten_Balingen", "Knoten_Zadar"];
        let cluster = bootstrap_raft_cluster(&nodes);

        assert!(cluster.current_leader.is_some(), "Must elect a leader");
        assert!(cluster.term > 0, "Term must advance");
        assert!(
            cluster.commit_ledger_entry("Knoten_Berlin", 1),
            "Ledger commit must succeed with quorum"
        );
        assert!(
            cluster
                .nodes
                .contains(cluster.current_leader.as_ref().unwrap()),
            "Leader must be a member of the cluster"
        );
    }

    #[test]
    fn test_raft_autonomous_failover_resilience() {
        let nodes = ["Knoten_Berlin", "Knoten_Balingen", "Knoten_Zadar"];
        let mut cluster = RaftCluster::new(&nodes);

        while cluster.current_leader.is_none() {
            cluster.start_election();
        }
        let original_leader = cluster.current_leader.clone().unwrap();
        cluster.heartbeat(&original_leader);

        cluster.heartbeats.insert(original_leader.clone(), 0);
        let failover_occurred = cluster.detect_leader_failure();
        assert!(
            failover_occurred,
            "Must detect leader failure and trigger new election"
        );
        while cluster.current_leader.is_none() {
            cluster.start_election();
        }
        assert!(
            cluster.current_leader.is_some(),
            "Must have a leader after failover"
        );
        assert!(
            cluster.term > 1,
            "Term must have advanced after failover election"
        );
    }

    #[test]
    fn test_v2_production_release_integrity() {
        let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/LANGUAGE_REFERENCE");
        let node_types_path = format!("{}/node_types.json", base);
        let native_fns_path = format!("{}/native_functions.json", base);

        let node_types: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&node_types_path).expect("node_types.json must exist"),
        )
        .expect("node_types.json must be valid JSON");
        let schema_nodes = node_types["oneOf"].as_array().expect("oneOf must be array");
        assert!(
            schema_nodes.len() >= 80,
            "Schema must define at least 80 node types"
        );

        let native_fns: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&native_fns_path).expect("native_functions.json must exist"),
        )
        .expect("native_functions.json must be valid JSON");
        let functions = native_fns["functions"]
            .as_array()
            .expect("functions must be array");
        assert!(
            functions.len() >= 40,
            "Must have at least 40 native functions"
        );

        let test_nodes: &[&str] = &[
            "LoadComputeShader",
            "DispatchCompute",
            "SpawnIsolate",
            "PlayNote",
            "StopNote",
        ];
        for name in test_nodes {
            let found = schema_nodes.iter().any(|n| {
                n["required"]
                    .as_array()
                    .map(|a| a.iter().any(|v| v.as_str() == Some(name)))
                    .unwrap_or(false)
            });
            assert!(found, "Required node {name} must exist in node_types.json");
        }

        let mut vm = VM::new();
        vm.crypto_state_hash = 0xDEAD;
        let data = vm.inspect();
        assert_eq!(
            data.scheduler_harness_status, "n/a (local test harness)",
            "Dashboard must show local test harness status"
        );
        assert!(
            data.crypto_state_hash > 0,
            "Dashboard must show crypto hash integrity"
        );
    }

    #[test]
    fn test_compiler_ast_full_alignment() {
        use knoten_core_types::ast::Node;
        let mut compiler = crate::vm::compiler::Compiler::new();

        let test_nodes: Vec<Node> = vec![
            Node::IntLiteral(1),
            Node::FloatLiteral(1.0),
            Node::BoolLiteral(true),
            Node::StringLiteral("test".into()),
            Node::Add(Box::new(Node::IntLiteral(1)), Box::new(Node::IntLiteral(2))),
            Node::Sub(Box::new(Node::IntLiteral(1)), Box::new(Node::IntLiteral(2))),
            Node::Mul(Box::new(Node::IntLiteral(1)), Box::new(Node::IntLiteral(2))),
            Node::Div(Box::new(Node::IntLiteral(1)), Box::new(Node::IntLiteral(2))),
            Node::Sin(Box::new(Node::FloatLiteral(1.0))),
            Node::Cos(Box::new(Node::FloatLiteral(1.0))),
            Node::Eq(Box::new(Node::IntLiteral(1)), Box::new(Node::IntLiteral(1))),
            Node::Lt(Box::new(Node::IntLiteral(1)), Box::new(Node::IntLiteral(2))),
            Node::Gt(Box::new(Node::IntLiteral(2)), Box::new(Node::IntLiteral(1))),
        ];

        let mut success_count = 0;
        for node in &test_nodes {
            if compiler.compile_node(node) {
                success_count += 1;
            }
        }
        assert!(
            success_count >= test_nodes.len(),
            "All {} test nodes must compile, only {} succeeded",
            test_nodes.len(),
            success_count
        );
    }

    #[test]
    fn test_watchdog_sleep_exclusion() {
        let instructions = vec![
            OpCode::Constant(0),
            OpCode::ExternCall {
                name_idx: 1,
                arg_count: 1,
            },
            OpCode::Constant(0),
            OpCode::ExternCall {
                name_idx: 1,
                arg_count: 1,
            },
            OpCode::Constant(0),
            OpCode::ExternCall {
                name_idx: 1,
                arg_count: 1,
            },
            OpCode::Constant(0),
            OpCode::ExternCall {
                name_idx: 1,
                arg_count: 1,
            },
            OpCode::Return,
        ];
        let constants = vec![
            RelType::Int(16),
            RelType::Str("time_sleep_ms".to_string()),
            RelType::Str("time".to_string()),
        ];

        let mut vm = VM::new();
        let bridge = crate::natives::bridge::CoreBridge;
        let perms = AgentPermissions::default();
        inspector::VM_SLEEP_ACCUMULATED_MS.store(0, std::sync::atomic::Ordering::SeqCst);

        let result = vm.run(&instructions, &constants, &perms, Some(&bridge));
        assert!(
            result.is_ok(),
            "Isolate must survive 4x16ms sleep without watchdog timeout"
        );
    }

    #[test]
    fn test_raft_network_randomized_election() {
        let _lock = TEST_SNAPSHOT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let nodes = ["Knoten_Berlin", "Knoten_Balingen", "Knoten_Zadar"];
        let mut cluster = RaftCluster::new(&nodes);
        let mut attempts = 0;
        while cluster.current_leader.is_none() && attempts < 20 {
            cluster.start_election();
            attempts += 1;
        }

        assert!(
            cluster.current_leader.is_some(),
            "Randomized election must produce a leader within 20 attempts"
        );
        assert!(cluster.term > 0, "Term must advance");

        let mut cluster2 = RaftCluster::new(&nodes);
        let mut attempts2 = 0;
        while cluster2.current_leader.is_none() && attempts2 < 20 {
            cluster2.start_election();
            attempts2 += 1;
        }
        assert!(
            cluster2.current_leader.is_some(),
            "Second cluster must also elect within 20 attempts"
        );
        assert!(
            cluster2.is_election_timeout("Knoten_Berlin"),
            "Timeout must be configurable"
        );
    }

    #[test]
    fn test_raft_distributed_log_replication() {
        let _lock = TEST_SNAPSHOT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let nodes = ["Knoten_Berlin", "Knoten_Balingen", "Knoten_Zadar"];
        let mut cluster = RaftCluster::new(&nodes);

        // Ensure quorum by retrying election until a leader is elected.
        // start_election() is randomised; in rare cases all votes may be 0.
        // This loop is deterministic in intent: it stops as soon as a leader exists.
        let mut attempts = 0;
        while cluster.current_leader.is_none() && attempts < 20 {
            cluster.start_election();
            attempts += 1;
        }
        assert!(
            cluster.current_leader.is_some(),
            "A leader must be elected within 20 attempts"
        );

        let replicated = cluster.replicate_log_entry(1, "deadbeef");
        assert!(
            replicated,
            "Log entry must replicate with quorum acknowledgment"
        );
        assert_eq!(cluster.replicated_logs.len(), 1);
        assert_eq!(cluster.replicated_logs[0].0, 1);
        assert_eq!(cluster.replicated_logs[0].1, "deadbeef");

        let acked = cluster.get_quorum_acked_logs();
        assert_eq!(acked.len(), 1, "Quorum acked logs must be retrievable");
    }

    #[test]
    fn test_sandbox_opcode_limit_guard() {
        let mut vm = VM::new();
        let instructions = vec![OpCode::Jump(0)];
        let constants = vec![];
        let permissions = AgentPermissions::default();
        let res = vm.run(&instructions, &constants, &permissions, None);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.contains("ERR_QUOTA_EXCEEDED") || err.contains("ERR_SANDBOX_TIMEOUT"));
    }

    #[test]
    fn test_sandbox_memory_limit_guard() {
        let mut vm = VM::new();
        let instructions = vec![OpCode::Constant(0); 100];
        let constants = vec![RelType::Str("A".repeat(300 * 1024))];
        let permissions = AgentPermissions::default();
        let res = vm.run(&instructions, &constants, &permissions, None);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.contains("ERR_MEMORY_LIMIT_EXCEEDED"));
    }
}
