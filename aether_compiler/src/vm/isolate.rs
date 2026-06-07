use super::machine::VM;
use super::scheduler::try_steal_work;
use super::snapshot::{snapshot_isolate, store_snapshot};
use crate::executor::{AgentPermissions, RelType};
use knoten_core_types::opcode::OpCode;
use std::collections::HashMap;

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
        let perms = AgentPermissions::default();
        if self.instructions.is_empty()
            && let Some(stolen) = try_steal_work(self.isolate_id)
        {
            self.instructions = vec![stolen.0];
            self.constants = stolen.1;
        }
        if self.isolate_id >= 0 {
            store_snapshot(self.isolate_id, vm.snapshot());
        }
        match vm.run(&self.instructions, &self.constants, &perms, None) {
            Ok(value) => Ok(value),
            Err(e) => {
                if self.isolate_id >= 0
                    && let Some(snapshot) = snapshot_isolate(self.isolate_id)
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
