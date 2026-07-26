// Sprint 307: Optimizer Constant Folding & Dead Code Trimming Tests
//
// Tests verify:
//   1. Node::ToInt folding (Int, Float, Bool, valid/invalid Str)
//   2. Node::ToFloat folding (Float, Int, Bool, valid/invalid Str)
//   3. Node::StringConcat & Node::StringContains constant folding
//   4. Dead branch elimination in Node::If after condition folding
//   5. Execution parity between unoptimized and optimized ASTs in the bare-metal Stack-VM

use aether_compiler::executor::{AgentPermissions, RelType};
use aether_compiler::optimizer::optimize;
use aether_compiler::vm::compiler::Compiler;
use aether_compiler::vm::machine::VM;
use knoten_core_types::ast::Node;

fn sandbox_perms() -> AgentPermissions {
    AgentPermissions {
        allow_network: false,
        allowed_domains: vec![],
        allow_fs_read: false,
        allow_fs_write: false,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Cast Constant Folding — ToInt
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_optimize_to_int_float_literal() {
    let unoptimized = Node::ToInt(Box::new(Node::FloatLiteral(4.9)));
    let optimized = optimize(unoptimized);
    assert_eq!(optimized, Node::IntLiteral(4));
}

#[test]
fn test_optimize_to_int_bool_literal() {
    let unoptimized = Node::ToInt(Box::new(Node::BoolLiteral(true)));
    let optimized = optimize(unoptimized);
    assert_eq!(optimized, Node::IntLiteral(1));

    let unoptimized_false = Node::ToInt(Box::new(Node::BoolLiteral(false)));
    let optimized_false = optimize(unoptimized_false);
    assert_eq!(optimized_false, Node::IntLiteral(0));
}

#[test]
fn test_optimize_to_int_str_literal_valid() {
    let unoptimized = Node::ToInt(Box::new(Node::StringLiteral(" 123 ".to_string())));
    let optimized = optimize(unoptimized);
    assert_eq!(optimized, Node::IntLiteral(123));
}

#[test]
fn test_optimize_to_int_str_literal_invalid_passes_through() {
    let unoptimized = Node::ToInt(Box::new(Node::StringLiteral("invalid".to_string())));
    let optimized = optimize(unoptimized.clone());
    // Invalid strings cannot be folded at compile time, must pass through
    assert_eq!(optimized, unoptimized);
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Cast Constant Folding — ToFloat
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_optimize_to_float_int_literal() {
    let unoptimized = Node::ToFloat(Box::new(Node::IntLiteral(42)));
    let optimized = optimize(unoptimized);
    assert_eq!(optimized, Node::FloatLiteral(42.0));
}

#[test]
fn test_optimize_to_float_bool_literal() {
    let unoptimized = Node::ToFloat(Box::new(Node::BoolLiteral(true)));
    let optimized = optimize(unoptimized);
    assert_eq!(optimized, Node::FloatLiteral(1.0));
}

#[test]
fn test_optimize_to_float_str_literal_valid() {
    let unoptimized = Node::ToFloat(Box::new(Node::StringLiteral("2.5".to_string())));
    let optimized = optimize(unoptimized);
    assert_eq!(optimized, Node::FloatLiteral(2.5));
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. String Primitive Constant Folding
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_optimize_string_concat_literals() {
    let unoptimized = Node::StringConcat(
        Box::new(Node::StringLiteral("Knoten".to_string())),
        Box::new(Node::StringLiteral("Core".to_string())),
    );
    let optimized = optimize(unoptimized);
    assert_eq!(optimized, Node::StringLiteral("KnotenCore".to_string()));
}

#[test]
fn test_optimize_string_concat_string_and_scalar() {
    let unoptimized = Node::StringConcat(
        Box::new(Node::StringLiteral("Sprint ".to_string())),
        Box::new(Node::IntLiteral(307)),
    );
    let optimized = optimize(unoptimized);
    assert_eq!(optimized, Node::StringLiteral("Sprint 307".to_string()));
}

#[test]
fn test_optimize_string_contains_literals() {
    let unoptimized_true = Node::StringContains(
        Box::new(Node::StringLiteral("KnotenCore v2.5.0-opt".to_string())),
        Box::new(Node::StringLiteral("v2.5.0".to_string())),
    );
    assert_eq!(optimize(unoptimized_true), Node::BoolLiteral(true));

    let unoptimized_false = Node::StringContains(
        Box::new(Node::StringLiteral("KnotenCore".to_string())),
        Box::new(Node::StringLiteral("RustVM".to_string())),
    );
    assert_eq!(optimize(unoptimized_false), Node::BoolLiteral(false));
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Dead Code Trimming after Condition Folding
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_optimize_if_prunes_dead_else_branch() {
    // StringContains folds to true -> If prunes else branch completely
    let cond = Node::StringContains(
        Box::new(Node::StringLiteral("hello world".to_string())),
        Box::new(Node::StringLiteral("world".to_string())),
    );
    let unoptimized = Node::If(
        Box::new(cond),
        Box::new(Node::IntLiteral(100)),
        Some(Box::new(Node::IntLiteral(999))),
    );
    let optimized = optimize(unoptimized);
    assert_eq!(
        optimized,
        Node::IntLiteral(100),
        "Dead else branch must be eliminated"
    );
}

#[test]
fn test_optimize_if_prunes_dead_then_branch() {
    // StringContains folds to false -> If prunes then branch completely
    let cond = Node::StringContains(
        Box::new(Node::StringLiteral("hello world".to_string())),
        Box::new(Node::StringLiteral("missing".to_string())),
    );
    let unoptimized = Node::If(
        Box::new(cond),
        Box::new(Node::IntLiteral(999)),
        Some(Box::new(Node::IntLiteral(200))),
    );
    let optimized = optimize(unoptimized);
    assert_eq!(
        optimized,
        Node::IntLiteral(200),
        "Dead then branch must be eliminated"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Parity & Opcode Reduction Verification
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_optimizer_opcode_reduction_and_execution_parity() {
    // Build a nested AST: StringConcat("Count: ", ToInt(ToFloat(10)))
    let ast = Node::StringConcat(
        Box::new(Node::StringLiteral("Count: ".to_string())),
        Box::new(Node::ToInt(Box::new(Node::ToFloat(Box::new(
            Node::IntLiteral(10),
        ))))),
    );

    // Compile unoptimized
    let mut compiler1 = Compiler::new();
    compiler1.compile_node(&ast);
    let unopt_instructions = compiler1.instructions.clone();
    let unopt_constants = compiler1.constants.clone();

    // Optimize AST
    let opt_ast = optimize(ast);

    // Compile optimized
    let mut compiler2 = Compiler::new();
    compiler2.compile_node(&opt_ast);
    let opt_instructions = compiler2.instructions.clone();
    let opt_constants = compiler2.constants.clone();

    // Verification 1: Optimized AST generates FEWER opcodes
    assert!(
        opt_instructions.len() < unopt_instructions.len(),
        "Optimized AST must generate fewer opcodes (unoptimized: {}, optimized: {})",
        unopt_instructions.len(),
        opt_instructions.len()
    );

    // Verification 2: Optimized AST folds down to a single Constant instruction!
    assert_eq!(
        opt_ast,
        Node::StringLiteral("Count: 10".to_string()),
        "Entire expression must fold to StringLiteral('Count: 10')"
    );

    // Verification 3: Execution results are identical on VM
    let mut vm1 = VM::default();
    let res1 = vm1
        .run(
            &unopt_instructions,
            &unopt_constants,
            &sandbox_perms(),
            None,
        )
        .unwrap();

    let mut vm2 = VM::default();
    let res2 = vm2
        .run(&opt_instructions, &opt_constants, &sandbox_perms(), None)
        .unwrap();

    assert_eq!(res1, res2, "Execution parity must hold!");
    assert_eq!(res2, RelType::Str("Count: 10".to_string()));
}
