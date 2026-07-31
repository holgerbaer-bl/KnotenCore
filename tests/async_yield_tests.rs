// Sprint 309: Async Yield, Non-blocking VM Suspension & Resuming Integration Tests
//
// Tests verify:
//   1. OpCode::Yield suspends execution and sets VmExecutionState::Yielded
//   2. vm.is_yielded() returns true when paused
//   3. vm.resume(...) seamless continuation from exact saved IP
//   4. Calling resume() when state is not Yielded returns Error
//   5. Multiple Yield opcodes in a single script resume sequentially
//   6. Compiled Node::Yield AST pipeline execution parity

use aether_compiler::executor::{AgentPermissions, RelType};
use aether_compiler::vm::compiler::Compiler;
use aether_compiler::vm::machine::{VM, VmExecutionState};
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
fn test_yield_opcode_suspends_and_resumes() {
    let mut vm = VM::default();
    assert_eq!(*vm.execution_state(), VmExecutionState::Ready);

    // Constants: [0: "x", 1: 100, 2: 50]
    // Script: Constant(1) -> SetGlobal(0) -> Yield -> GetGlobal(0) + Constant(2) -> Add -> Return
    let constants = vec![
        RelType::Str("x".to_string()),
        RelType::Int(100),
        RelType::Int(50),
    ];
    let instructions = vec![
        OpCode::Constant(1),
        OpCode::SetGlobal(0),
        OpCode::Yield,
        OpCode::GetGlobal(0),
        OpCode::Constant(2),
        OpCode::Add,
        OpCode::Return,
    ];

    // Execution step 1: Run until Yield
    let res1 = vm.run(&instructions, &constants, &sandbox_perms(), None).unwrap();
    assert_eq!(res1, RelType::Void);
    assert!(vm.is_yielded(), "VM must be in Yielded state after OpCode::Yield");
    assert_eq!(*vm.execution_state(), VmExecutionState::Yielded);

    // Globals set before Yield must be preserved
    assert_eq!(vm.globals.get("x"), Some(&RelType::Int(100)));

    // Execution step 2: Resume VM
    let res2 = vm.resume(&instructions, &constants, &sandbox_perms(), None).unwrap();
    assert_eq!(res2, RelType::Int(150), "Resumed execution must compute 100 + 50 = 150");
    assert_eq!(*vm.execution_state(), VmExecutionState::Finished(RelType::Int(150)));
}

#[test]
fn test_multiple_sequential_yields() {
    let mut vm = VM::default();

    // Constants: [0: "x", 1: 1, 2: 2]
    // Script: Constant(1) -> SetGlobal(0) -> Yield -> Constant(2) -> SetGlobal(0) -> Yield -> GetGlobal(0) -> Return
    let constants = vec![
        RelType::Str("x".to_string()),
        RelType::Int(1),
        RelType::Int(2),
    ];
    let instructions = vec![
        OpCode::Constant(1),
        OpCode::SetGlobal(0),
        OpCode::Yield,
        OpCode::Constant(2),
        OpCode::SetGlobal(0),
        OpCode::Yield,
        OpCode::GetGlobal(0),
        OpCode::Return,
    ];

    // Yield 1
    vm.run(&instructions, &constants, &sandbox_perms(), None).unwrap();
    assert!(vm.is_yielded());
    assert_eq!(vm.globals.get("x"), Some(&RelType::Int(1)));

    // Yield 2
    vm.resume(&instructions, &constants, &sandbox_perms(), None).unwrap();
    assert!(vm.is_yielded());
    assert_eq!(vm.globals.get("x"), Some(&RelType::Int(2)));

    // Finish
    let final_res = vm.resume(&instructions, &constants, &sandbox_perms(), None).unwrap();
    assert_eq!(final_res, RelType::Int(2));
    assert!(!vm.is_yielded());
}

#[test]
fn test_resume_non_yielded_vm_returns_error() {
    let mut vm = VM::default();
    let instructions = vec![OpCode::Constant(0), OpCode::Return];
    let constants = vec![RelType::Int(42)];

    // Running a normal script that finishes
    let res = vm.run(&instructions, &constants, &sandbox_perms(), None).unwrap();
    assert_eq!(res, RelType::Int(42));
    assert_eq!(*vm.execution_state(), VmExecutionState::Finished(RelType::Int(42)));

    // Attempting to resume a Finished VM should return Error
    let resume_res = vm.resume(&instructions, &constants, &sandbox_perms(), None);
    assert!(resume_res.is_err(), "Resume on non-yielded VM must fail");
    assert!(matches!(*vm.execution_state(), VmExecutionState::Fault(_)));
}

#[test]
fn test_compiled_node_yield_ast_pipeline() {
    let mut vm = VM::default();

    // AST: Assign("x", 10); Yield; Assign("x", 20); Return "x"
    let ast = Node::Block(vec![
        Node::Assign("x".to_string(), Box::new(Node::IntLiteral(10))),
        Node::Yield,
        Node::Assign("x".to_string(), Box::new(Node::IntLiteral(20))),
        Node::Identifier("x".to_string()),
    ]);

    let mut compiler = Compiler::new();
    compiler.compile_node(&ast);

    // Initial run hits Yield
    vm.run(&compiler.instructions, &compiler.constants, &sandbox_perms(), None).unwrap();
    assert!(vm.is_yielded());
    assert_eq!(vm.globals.get("x"), Some(&RelType::Int(10)));

    // Resume completes script
    let final_val = vm
        .resume(&compiler.instructions, &compiler.constants, &sandbox_perms(), None)
        .unwrap();

    assert_eq!(final_val, RelType::Int(20));
    assert_eq!(vm.globals.get("x"), Some(&RelType::Int(20)));
}
