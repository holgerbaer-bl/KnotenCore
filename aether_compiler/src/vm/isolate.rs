use super::machine::{VM, VMState};
use super::scheduler::try_steal_work;
use super::snapshot::{snapshot_isolate, store_snapshot};
use crate::executor::{AgentPermissions, RelType};
use knoten_core_types::opcode::OpCode;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

// Sprint 270: Hot-swap registry for live instruction mutation
type IsolateCode = Arc<Mutex<(Vec<OpCode>, Vec<RelType>)>>;
static HOT_SWAP_REGISTRY: OnceLock<Mutex<HashMap<i64, IsolateCode>>> = OnceLock::new();

pub(super) fn get_hot_swap_registry() -> &'static Mutex<HashMap<i64, IsolateCode>> {
    HOT_SWAP_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn hot_swap_isolate_code(
    isolate_id: i64,
    new_instructions: Vec<OpCode>,
    new_constants: Vec<RelType>,
) -> bool {
    let registry = get_hot_swap_registry();
    let guard = registry.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(code) = guard.get(&isolate_id) {
        let mut locked = code.lock().unwrap_or_else(|e| e.into_inner());
        store_snapshot(isolate_id, VM::new().snapshot());
        *locked = (new_instructions, new_constants);
        true
    } else {
        false
    }
}

pub fn drain_hot_swap_registry() {
    get_hot_swap_registry()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
}

// Sprint 279: Adaptive evolutionary PGO — dynamic instruction mutation
pub fn optimize_active_hotpath(isolate_id: i64) -> bool {
    let registry = get_hot_swap_registry();
    let guard = registry.lock().unwrap_or_else(|e| e.into_inner());
    let code = match guard.get(&isolate_id) {
        Some(c) => Arc::clone(c),
        None => return false,
    };
    drop(guard);

    let mut locked = code.lock().unwrap_or_else(|e| e.into_inner());
    let (ref current_instrs, ref current_consts) = *locked;

    let mut hot_ips: Vec<usize> = super::machine::HOT_PATH_TABLE.with(|t| {
        let table = t.borrow();
        table
            .iter()
            .filter(|&(_, count)| *count >= 10_000)
            .map(|(&ip, _)| ip)
            .collect()
    });
    hot_ips.sort();

    if hot_ips.is_empty() {
        return false;
    }

    let mut new_instructions = current_instrs.clone();
    let mut modified = false;

    for &hot_ip in &hot_ips {
        if hot_ip < current_instrs.len() {
            match &current_instrs[hot_ip] {
                OpCode::Jump(_) | OpCode::JumpIfFalse(_) => {
                    unroll_loop_at(&mut new_instructions, hot_ip, current_instrs);
                    modified = true;
                }
                _ => {}
            }
        }
    }

    if modified {
        store_snapshot(isolate_id, VM::new().snapshot());
        *locked = (new_instructions, current_consts.clone());
        telemetry_push(
            isolate_id,
            format!("PGO: optimized hotpath at IPs {:?}", hot_ips),
        );
        true
    } else {
        false
    }
}

fn unroll_loop_at(instructions: &mut Vec<OpCode>, jump_ip: usize, _original: &[OpCode]) {
    let len = instructions.len();
    if len < 3 || jump_ip >= len {
        return;
    }
    let jump_target = match &instructions[jump_ip] {
        OpCode::Jump(t) => *t,
        _ => return,
    };
    if jump_target >= jump_ip {
        return;
    }
    let body: Vec<OpCode> = instructions[jump_target..jump_ip].to_vec();
    let unroll_factor = 2;
    let body_len = body.len();

    for copy_idx in 0..unroll_factor {
        let mut cloned_body = body.clone();
        let _copy_offset = jump_ip + copy_idx * body_len;
        for instr in &mut cloned_body {
            match instr {
                OpCode::Jump(t) | OpCode::JumpIfFalse(t) if *t >= jump_ip => {
                    *t += body_len;
                }
                OpCode::Jump(_) | OpCode::JumpIfFalse(_) => {}
                _ => {}
            }
        }
        instructions.splice(jump_ip..jump_ip, cloned_body);
    }
    relocate_jumps(instructions, jump_ip, body_len * unroll_factor);
}

fn relocate_jumps(instructions: &mut [OpCode], insert_pos: usize, shift_amount: usize) {
    for instr in instructions.iter_mut() {
        match instr {
            OpCode::Jump(target) | OpCode::JumpIfFalse(target) if *target >= insert_pos => {
                *target += shift_amount;
            }
            _ => {}
        }
    }
}

// Sprint 271: Agent telemetry channel for structured runtime diagnostics
static AGENT_TELEMETRY: OnceLock<dashmap::DashMap<i64, Vec<String>>> = OnceLock::new();

fn get_telemetry() -> &'static dashmap::DashMap<i64, Vec<String>> {
    AGENT_TELEMETRY.get_or_init(dashmap::DashMap::new)
}

pub fn telemetry_push(isolate_id: i64, diagnostic: String) {
    get_telemetry()
        .entry(isolate_id)
        .or_default()
        .push(diagnostic);
}

pub fn telemetry_last(isolate_id: i64) -> Option<String> {
    get_telemetry()
        .get(&isolate_id)
        .and_then(|v| v.last().cloned())
}

pub fn telemetry_drain(isolate_id: i64) -> Vec<String> {
    get_telemetry()
        .remove(&isolate_id)
        .map(|(_, v)| v)
        .unwrap_or_default()
}

// Sprint 272: Lockless shared-memory virtual buses (inter-isolate DMA)
static VIRTUAL_BUSES: OnceLock<dashmap::DashMap<String, Arc<Vec<RelType>>>> = OnceLock::new();

fn get_buses() -> &'static dashmap::DashMap<String, Arc<Vec<RelType>>> {
    VIRTUAL_BUSES.get_or_init(dashmap::DashMap::new)
}

pub fn bus_publish(name: String, data: Vec<RelType>) {
    get_buses().insert(name, Arc::new(data));
}

pub fn bus_subscribe(name: &str) -> Option<Arc<Vec<RelType>>> {
    get_buses().get(name).map(|r| Arc::clone(&*r))
}

pub fn bus_drain() {
    get_buses().clear();
}

pub struct VMIsolate {
    pub instructions: Vec<OpCode>,
    pub constants: Vec<RelType>,
    pub isolate_id: i64,
    pub mailbox: Option<std::sync::mpsc::Receiver<RelType>>,
    pub local_heap: HashMap<String, RelType>,
}

impl VMIsolate {
    pub fn new(instructions: Vec<OpCode>, constants: Vec<RelType>) -> Self {
        Self {
            instructions,
            constants,
            isolate_id: -1,
            mailbox: None,
            local_heap: HashMap::new(),
        }
    }

    pub fn with_mailbox(
        instructions: Vec<OpCode>,
        constants: Vec<RelType>,
        isolate_id: i64,
        mailbox: std::sync::mpsc::Receiver<RelType>,
    ) -> Self {
        Self {
            instructions,
            constants,
            isolate_id,
            mailbox: Some(mailbox),
            local_heap: HashMap::new(),
        }
    }

    pub fn run(mut self) -> Result<RelType, String> {
        let mut vm = VM::new();
        for (k, v) in self.local_heap.drain() {
            vm.globals.insert(k, v);
        }

        let swap_id = self.isolate_id;
        let code: IsolateCode = if swap_id >= 0 {
            let registry = get_hot_swap_registry();
            let guard = registry.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(existing) = guard.get(&swap_id) {
                Arc::clone(existing)
            } else {
                drop(guard);
                let new_code = Arc::new(Mutex::new((
                    self.instructions.clone(),
                    self.constants.clone(),
                )));
                get_hot_swap_registry()
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(swap_id, Arc::clone(&new_code));
                new_code
            }
        } else {
            Arc::new(Mutex::new((
                self.instructions.clone(),
                self.constants.clone(),
            )))
        };

        let (initial_instr, initial_const) = code.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let mut instructions = initial_instr;
        let mut constants = initial_const;

        let perms = AgentPermissions::default();
        if instructions.is_empty()
            && let Some(stolen) = try_steal_work(swap_id)
        {
            instructions = vec![stolen.0];
            constants = stolen.1;
        }
        if swap_id >= 0 {
            store_snapshot(swap_id, vm.snapshot());
        }
        match vm.run(&instructions, &constants, &perms, None) {
            Ok(value) => Ok(value),
            Err(e) => {
                if swap_id >= 0
                    && let Some(snapshot) = snapshot_isolate(swap_id)
                {
                    vm.rollback(snapshot);
                }
                Err(e)
            }
        }
    }
}

pub fn spawn_isolate(
    instructions: Vec<OpCode>,
    constants: Vec<RelType>,
) -> std::thread::JoinHandle<Result<RelType, String>> {
    std::thread::spawn(move || {
        let isolate = VMIsolate::new(instructions, constants);
        isolate.run()
    })
}

// Sprint 274: Speculative shadow isolate — runs a code path from a frozen VM snapshot
pub struct SpeculativeResult {
    pub state: VMState,
    pub value: RelType,
}

pub fn spawn_shadow_isolate(
    snapshot: VMState,
    instructions: Vec<OpCode>,
    constants: Vec<RelType>,
) -> std::thread::JoinHandle<SpeculativeResult> {
    std::thread::spawn(move || {
        let mut vm = VM::new();
        vm.rollback(snapshot);
        let perms = AgentPermissions::default();
        match vm.run(&instructions, &constants, &perms, None) {
            Ok(value) => SpeculativeResult {
                state: vm.snapshot(),
                value,
            },
            Err(_) => SpeculativeResult {
                state: VM::new().snapshot(),
                value: RelType::Void,
            },
        }
    })
}
