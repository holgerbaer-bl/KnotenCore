use super::machine::VM;
use super::scheduler::try_steal_work;
use super::snapshot::{snapshot_isolate, store_snapshot};
use crate::executor::{AgentPermissions, RelType};
use knoten_core_types::opcode::OpCode;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

// Sprint 270: Hot-swap registry for live instruction mutation
type IsolateCode = Arc<Mutex<(Vec<OpCode>, Vec<RelType>)>>;
static HOT_SWAP_REGISTRY: OnceLock<Mutex<HashMap<i64, IsolateCode>>> = OnceLock::new();

fn get_hot_swap_registry() -> &'static Mutex<HashMap<i64, IsolateCode>> {
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
