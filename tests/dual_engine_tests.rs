use aether_compiler::executor::RelType;
use aether_compiler::rpc::KNC_PROTOCOL_VERSION;
use aether_compiler::vm::dual_validator::{
    DualEngineValidator, DualValidationOutcome, FaultCategory,
};
use knoten_core_types::ast::Node;

#[test]
fn test_version_assertion_sprint355() {
    assert_eq!(KNC_PROTOCOL_VERSION, "v2.24.18");
}

#[test]
fn test_dual_engine_arithmetic_parity() {
    let validator = DualEngineValidator::new();

    // 1. Basic Arithmetic: (10 + 20) * 3 - 15 / 3 = 85
    let ast = Node::Sub(
        Box::new(Node::Mul(
            Box::new(Node::Add(
                Box::new(Node::IntLiteral(10)),
                Box::new(Node::IntLiteral(20)),
            )),
            Box::new(Node::IntLiteral(3)),
        )),
        Box::new(Node::Div(
            Box::new(Node::IntLiteral(15)),
            Box::new(Node::IntLiteral(3)),
        )),
    );

    let res = validator.assert_parity(&ast).expect("Parity must hold");
    match res {
        DualValidationOutcome::Success { return_value, .. } => {
            assert_eq!(return_value, RelType::Int(85));
        }
        _ => panic!("Expected successful parity"),
    }
}

#[test]
fn test_dual_engine_float_arithmetic_parity() {
    let validator = DualEngineValidator::new();

    let ast = Node::Add(
        Box::new(Node::FloatLiteral(12.5)),
        Box::new(Node::Mul(
            Box::new(Node::FloatLiteral(4.0)),
            Box::new(Node::FloatLiteral(2.5)),
        )),
    );

    let res = validator.assert_parity(&ast).expect("Parity must hold");
    match res {
        DualValidationOutcome::Success { return_value, .. } => {
            assert_eq!(return_value, RelType::Float(22.5));
        }
        _ => panic!("Expected successful parity"),
    }
}

#[test]
fn test_dual_engine_control_flow_and_state_mutations() {
    let validator = DualEngineValidator::new();

    // Loop calculating sum 1..=10 and storing variables
    let ast = Node::Block(vec![
        Node::Assign("sum".to_string(), Box::new(Node::IntLiteral(0))),
        Node::Assign("i".to_string(), Box::new(Node::IntLiteral(1))),
        Node::While(
            Box::new(Node::Lt(
                Box::new(Node::Identifier("i".to_string())),
                Box::new(Node::IntLiteral(11)),
            )),
            Box::new(Node::Block(vec![
                Node::Assign(
                    "sum".to_string(),
                    Box::new(Node::Add(
                        Box::new(Node::Identifier("sum".to_string())),
                        Box::new(Node::Identifier("i".to_string())),
                    )),
                ),
                Node::Assign(
                    "i".to_string(),
                    Box::new(Node::Add(
                        Box::new(Node::Identifier("i".to_string())),
                        Box::new(Node::IntLiteral(1)),
                    )),
                ),
            ])),
        ),
        Node::Identifier("sum".to_string()),
    ]);

    let res = validator.assert_parity(&ast).expect("Parity must hold");
    match res {
        DualValidationOutcome::Success {
            return_value,
            state_mutations,
        } => {
            assert_eq!(return_value, RelType::Int(55));
            assert_eq!(state_mutations.get("sum"), Some(&RelType::Int(55)));
            assert_eq!(state_mutations.get("i"), Some(&RelType::Int(11)));
        }
        _ => panic!("Expected successful parity"),
    }
}

#[test]
fn test_dual_engine_fibonacci_parity() {
    let validator = DualEngineValidator::new();

    let fib_ast = Node::Block(vec![
        Node::Assign("n".to_string(), Box::new(Node::IntLiteral(15))),
        Node::Assign("a".to_string(), Box::new(Node::IntLiteral(0))),
        Node::Assign("b".to_string(), Box::new(Node::IntLiteral(1))),
        Node::Assign("i".to_string(), Box::new(Node::IntLiteral(0))),
        Node::While(
            Box::new(Node::Lt(
                Box::new(Node::Identifier("i".to_string())),
                Box::new(Node::Identifier("n".to_string())),
            )),
            Box::new(Node::Block(vec![
                Node::Assign(
                    "temp".to_string(),
                    Box::new(Node::Add(
                        Box::new(Node::Identifier("a".to_string())),
                        Box::new(Node::Identifier("b".to_string())),
                    )),
                ),
                Node::Assign("a".to_string(), Box::new(Node::Identifier("b".to_string()))),
                Node::Assign(
                    "b".to_string(),
                    Box::new(Node::Identifier("temp".to_string())),
                ),
                Node::Assign(
                    "i".to_string(),
                    Box::new(Node::Add(
                        Box::new(Node::Identifier("i".to_string())),
                        Box::new(Node::IntLiteral(1)),
                    )),
                ),
            ])),
        ),
        Node::Identifier("a".to_string()),
    ]);

    let res = validator
        .assert_parity(&fib_ast)
        .expect("Fibonacci parity must hold");
    match res {
        DualValidationOutcome::Success {
            return_value,
            state_mutations,
        } => {
            assert_eq!(return_value, RelType::Int(610)); // Fib(15) = 610
            assert_eq!(state_mutations.get("a"), Some(&RelType::Int(610)));
            assert_eq!(state_mutations.get("n"), Some(&RelType::Int(15)));
        }
        _ => panic!("Expected successful parity"),
    }
}

#[test]
fn test_dual_engine_fault_parity_division_by_zero() {
    let validator = DualEngineValidator::new();

    // 1. Integer division by zero
    let int_div_zero = Node::Div(
        Box::new(Node::IntLiteral(100)),
        Box::new(Node::IntLiteral(0)),
    );
    let report_int = validator.validate(&int_div_zero);
    assert!(report_int.is_valid);
    match report_int.outcome.unwrap() {
        DualValidationOutcome::SymmetricalFault { category, .. } => {
            assert_eq!(category, FaultCategory::DivisionByZero);
        }
        _ => panic!("Expected symmetrical division by zero fault"),
    }

    // 2. Float division by zero
    let float_div_zero = Node::Div(
        Box::new(Node::FloatLiteral(42.0)),
        Box::new(Node::FloatLiteral(0.0)),
    );
    let report_float = validator.validate(&float_div_zero);
    assert!(report_float.is_valid);
    match report_float.outcome.unwrap() {
        DualValidationOutcome::SymmetricalFault { category, .. } => {
            assert_eq!(category, FaultCategory::DivisionByZero);
        }
        _ => panic!("Expected symmetrical division by zero fault"),
    }
}

#[test]
fn test_dual_engine_fault_parity_modulo_by_zero() {
    let validator = DualEngineValidator::new();

    let mod_zero = Node::Modulo(
        Box::new(Node::IntLiteral(100)),
        Box::new(Node::IntLiteral(0)),
    );
    let report = validator.validate(&mod_zero);
    assert!(report.is_valid);
    match report.outcome.unwrap() {
        DualValidationOutcome::SymmetricalFault { category, .. } => {
            assert_eq!(category, FaultCategory::ModuloByZero);
        }
        _ => panic!("Expected symmetrical modulo by zero fault"),
    }
}

#[test]
fn test_dual_engine_fault_parity_type_mismatch() {
    let validator = DualEngineValidator::new();

    // Multiply integer with string literal
    let type_error_ast = Node::Mul(
        Box::new(Node::IntLiteral(10)),
        Box::new(Node::StringLiteral("invalid_text".to_string())),
    );

    let report = validator.validate(&type_error_ast);
    assert!(report.is_valid);
    match report.outcome.unwrap() {
        DualValidationOutcome::SymmetricalFault { category, .. } => {
            assert_eq!(category, FaultCategory::TypeError);
        }
        _ => panic!("Expected symmetrical type error fault"),
    }
}

#[test]
fn test_dual_engine_fault_parity_undefined_variable() {
    let validator = DualEngineValidator::new();

    let undef_ast = Node::Identifier("non_existent_var_12345".to_string());
    let report = validator.validate(&undef_ast);
    assert!(report.is_valid);
    match report.outcome.unwrap() {
        DualValidationOutcome::SymmetricalFault { category, .. } => {
            assert_eq!(category, FaultCategory::VariableNotFound);
        }
        _ => panic!("Expected symmetrical variable not found fault"),
    }
}

#[test]
fn test_dual_engine_gas_and_instruction_telemetry_decoupled() {
    let validator = DualEngineValidator::new();

    let ast = Node::Block(vec![
        Node::Assign("x".to_string(), Box::new(Node::IntLiteral(10))),
        Node::Assign("y".to_string(), Box::new(Node::IntLiteral(20))),
        Node::Add(
            Box::new(Node::Identifier("x".to_string())),
            Box::new(Node::Identifier("y".to_string())),
        ),
    ]);

    let report = validator.validate(&ast);
    assert!(report.is_valid);
    // Ensure gas consumption and instruction metrics are tracked without causing assertion failure
    assert!(report.vm_gas_consumed > 0);
    assert!(report.vm_instructions_executed > 0);
    assert!(report.eval_duration_ns > 0);
    assert!(report.vm_duration_ns > 0);
}

#[test]
fn test_dual_engine_zero_panic_containment() {
    let validator = DualEngineValidator::new();

    // Complex nested invalid operation that must not panic
    let bad_ast = Node::Block(vec![
        Node::Assign(
            "v".to_string(),
            Box::new(Node::Div(
                Box::new(Node::IntLiteral(1)),
                Box::new(Node::IntLiteral(0)),
            )),
        ),
        Node::Identifier("v".to_string()),
    ]);

    let report = validator.validate(&bad_ast);
    assert!(report.is_valid);
    assert!(report.outcome.is_some());
}
