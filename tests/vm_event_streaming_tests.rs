// Sprint 308: VM Event Streaming Hooks & EventEmit Opcode Integration Tests
//
// Tests verify:
//   1. OpCode::EventEmit / Node::EventEmit emits VmEvent::Custom to registered event hook
//   2. OpCode::VfsWrite emits VmEvent::VfsWrite to registered event hook
//   3. OpCode::VfsRead emits VmEvent::VfsRead to registered event hook
//   4. Thread-safe event collection via Arc<Mutex<Vec<VmEvent>>>

use std::sync::{Arc, Mutex};

use aether_compiler::executor::{AgentPermissions, RelType};
use aether_compiler::vm::compiler::Compiler;
use aether_compiler::vm::machine::{VM, VmEvent};
use knoten_core_types::ast::Node;
use knoten_core_types::opcode::OpCode;

fn sandbox_perms() -> AgentPermissions {
    AgentPermissions {
        allow_network: false,
        allowed_domains: vec![],
        allow_fs_read: false,
        allow_fs_write: false,
    }
}

#[test]
fn test_event_emit_opcode_custom_event_hook() {
    let mut vm = VM::default();
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = Arc::clone(&events);

    vm.set_event_hook(Arc::new(move |evt| {
        events_clone.lock().unwrap().push(evt);
    }));

    let c_topic = RelType::Str("telemetry.agent".to_string());
    let c_payload = RelType::Int(42);
    let constants = vec![c_topic, c_payload];
    let instructions = vec![
        OpCode::Constant(0),
        OpCode::Constant(1),
        OpCode::EventEmit,
        OpCode::Return,
    ];

    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Void);

    let collected = events.lock().unwrap();
    assert_eq!(collected.len(), 1);
    assert_eq!(
        collected[0],
        VmEvent::Custom {
            topic: "telemetry.agent".to_string(),
            payload: RelType::Int(42),
        }
    );
}

#[test]
fn test_vfs_write_and_read_event_hooks() {
    let mut vm = VM::default();
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = Arc::clone(&events);

    vm.set_event_hook(Arc::new(move |evt| {
        events_clone.lock().unwrap().push(evt);
    }));

    // Step 1: VfsWrite "/events/log.txt" "hello hooks"
    let c_path = RelType::Str("/events/log.txt".to_string());
    let c_data = RelType::Str("hello hooks".to_string());
    let constants = vec![c_path, c_data];
    let write_instructions = vec![
        OpCode::Constant(0),
        OpCode::Constant(1),
        OpCode::VfsWrite,
        OpCode::Return,
    ];

    vm.run(&write_instructions, &constants, &sandbox_perms(), None)
        .unwrap();

    // Step 2: VfsRead "/events/log.txt"
    let c_path2 = RelType::Str("/events/log.txt".to_string());
    let read_instructions = vec![OpCode::Constant(0), OpCode::VfsRead, OpCode::Return];

    vm.run(&read_instructions, &[c_path2], &sandbox_perms(), None)
        .unwrap();

    let collected = events.lock().unwrap();
    assert_eq!(collected.len(), 2, "Expected 2 events (VfsWrite, VfsRead)");
    assert_eq!(
        collected[0],
        VmEvent::VfsWrite {
            path: "/events/log.txt".to_string(),
            bytes: "hello hooks".len(),
        }
    );
    assert_eq!(
        collected[1],
        VmEvent::VfsRead {
            path: "/events/log.txt".to_string(),
        }
    );
}

#[test]
fn test_compiled_event_emit_node_pipeline() {
    let mut vm = VM::default();
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = Arc::clone(&events);

    vm.set_event_hook(Arc::new(move |evt| {
        events_clone.lock().unwrap().push(evt);
    }));

    // Build AST: Node::EventEmit("agent.status", "ONLINE")
    let ast = Node::EventEmit(
        Box::new(Node::StringLiteral("agent.status".to_string())),
        Box::new(Node::StringLiteral("ONLINE".to_string())),
    );

    let mut compiler = Compiler::new();
    compiler.compile_node(&ast);

    let result = vm
        .run(
            &compiler.instructions,
            &compiler.constants,
            &sandbox_perms(),
            None,
        )
        .unwrap();

    assert_eq!(result, RelType::Void);

    let collected = events.lock().unwrap();
    assert_eq!(collected.len(), 1);
    assert_eq!(
        collected[0],
        VmEvent::Custom {
            topic: "agent.status".to_string(),
            payload: RelType::Str("ONLINE".to_string()),
        }
    );
}
