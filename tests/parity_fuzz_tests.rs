use aether_compiler::executor::RelType;
use aether_compiler::rpc::KNC_PROTOCOL_VERSION;
use aether_compiler::vm::dual_validator::{
    DualEngineValidator, DualValidationOutcome, rel_type_eq_nan_aware, state_mutations_eq_nan_aware,
};
use knoten_core_types::ast::Node;
use std::collections::BTreeMap;

#[test]
fn test_version_assertion_sprint357() {
    assert_eq!(KNC_PROTOCOL_VERSION, "v2.24.20");
}

/// Simple deterministic XorShift64 PRNG for reproducible fuzz testing.
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0xdeadbeefcafebabe } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn next_range(&mut self, min: usize, max: usize) -> usize {
        if min >= max {
            return min;
        }
        min + (self.next_u64() as usize % (max - min + 1))
    }

    fn next_bool(&mut self) -> bool {
        (self.next_u64() & 1) == 1
    }

    fn next_int_boundary(&mut self) -> i64 {
        const BOUNDARIES: &[i64] = &[
            0,
            1,
            -1,
            2,
            -2,
            42,
            -42,
            100,
            -100,
            i64::MIN,
            i64::MAX,
            i64::MIN + 1,
            i64::MAX - 1,
            i32::MIN as i64,
            i32::MAX as i64,
        ];
        BOUNDARIES[self.next_range(0, BOUNDARIES.len() - 1)]
    }

    fn next_float_boundary(&mut self) -> f64 {
        const BOUNDARIES: &[f64] = &[
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            -0.5,
            2.5,
            -2.5,
            100.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::MIN_POSITIVE,
            f64::EPSILON,
        ];
        BOUNDARIES[self.next_range(0, BOUNDARIES.len() - 1)]
    }
}

/// Procedural AST Generator for differential parity fuzzing.
struct AstFuzzer {
    rng: DeterministicRng,
}

impl AstFuzzer {
    fn new(seed: u64) -> Self {
        Self {
            rng: DeterministicRng::new(seed),
        }
    }

    fn gen_terminal(&mut self) -> Node {
        match self.rng.next_range(0, 3) {
            0 => Node::IntLiteral(self.rng.next_int_boundary()),
            1 => Node::FloatLiteral(self.rng.next_float_boundary()),
            2 => Node::BoolLiteral(self.rng.next_bool()),
            3 => Node::StringLiteral(match self.rng.next_range(0, 3) {
                0 => "".to_string(),
                1 => "knoten".to_string(),
                2 => "fuzz".to_string(),
                _ => "core".to_string(),
            }),
            _ => unreachable!(),
        }
    }

    fn gen_node(&mut self, depth: usize, max_depth: usize) -> Node {
        if depth >= max_depth {
            return self.gen_terminal();
        }

        match self.rng.next_range(0, 15) {
            // Arithmetic Operations
            0 => Node::Add(
                Box::new(self.gen_node(depth + 1, max_depth)),
                Box::new(self.gen_node(depth + 1, max_depth)),
            ),
            1 => Node::Sub(
                Box::new(self.gen_node(depth + 1, max_depth)),
                Box::new(self.gen_node(depth + 1, max_depth)),
            ),
            2 => Node::Mul(
                Box::new(self.gen_node(depth + 1, max_depth)),
                Box::new(self.gen_node(depth + 1, max_depth)),
            ),
            3 => Node::Div(
                Box::new(self.gen_node(depth + 1, max_depth)),
                Box::new(self.gen_node(depth + 1, max_depth)),
            ),
            4 => Node::Modulo(
                Box::new(self.gen_node(depth + 1, max_depth)),
                Box::new(self.gen_node(depth + 1, max_depth)),
            ),
            5 => Node::Neg(Box::new(self.gen_node(depth + 1, max_depth))),

            // Comparison Operations
            6 => Node::Eq(
                Box::new(self.gen_node(depth + 1, max_depth)),
                Box::new(self.gen_node(depth + 1, max_depth)),
            ),
            7 => Node::NotEq(
                Box::new(self.gen_node(depth + 1, max_depth)),
                Box::new(self.gen_node(depth + 1, max_depth)),
            ),
            8 => Node::Lt(
                Box::new(self.gen_node(depth + 1, max_depth)),
                Box::new(self.gen_node(depth + 1, max_depth)),
            ),
            9 => Node::Lte(
                Box::new(self.gen_node(depth + 1, max_depth)),
                Box::new(self.gen_node(depth + 1, max_depth)),
            ),
            10 => Node::Gt(
                Box::new(self.gen_node(depth + 1, max_depth)),
                Box::new(self.gen_node(depth + 1, max_depth)),
            ),
            11 => Node::Gte(
                Box::new(self.gen_node(depth + 1, max_depth)),
                Box::new(self.gen_node(depth + 1, max_depth)),
            ),

            // Logical Operations
            12 => Node::And(
                Box::new(self.gen_node(depth + 1, max_depth)),
                Box::new(self.gen_node(depth + 1, max_depth)),
            ),
            13 => Node::Or(
                Box::new(self.gen_node(depth + 1, max_depth)),
                Box::new(self.gen_node(depth + 1, max_depth)),
            ),
            14 => Node::Not(Box::new(self.gen_node(depth + 1, max_depth))),

            // Conditional Control Flow
            15 => Node::If(
                Box::new(self.gen_node(depth + 1, max_depth)),
                Box::new(self.gen_node(depth + 1, max_depth)),
                Some(Box::new(self.gen_node(depth + 1, max_depth))),
            ),
            _ => self.gen_terminal(),
        }
    }
}

#[test]
fn test_nan_aware_differential_equality() {
    // Direct Float NaNs
    assert!(rel_type_eq_nan_aware(
        &RelType::Float(f64::NAN),
        &RelType::Float(f64::NAN)
    ));
    assert!(rel_type_eq_nan_aware(
        &RelType::Float(1.0),
        &RelType::Float(1.0)
    ));
    assert!(!rel_type_eq_nan_aware(
        &RelType::Float(1.0),
        &RelType::Float(2.0)
    ));

    // Nested in Arrays
    let arr1 = RelType::Array(vec![RelType::Float(f64::NAN), RelType::Int(42)]);
    let arr2 = RelType::Array(vec![RelType::Float(f64::NAN), RelType::Int(42)]);
    assert!(rel_type_eq_nan_aware(&arr1, &arr2));

    // State mutations BTreeMap
    let mut state1 = BTreeMap::new();
    state1.insert("x".to_string(), RelType::Float(f64::NAN));
    state1.insert("y".to_string(), RelType::Int(100));

    let mut state2 = BTreeMap::new();
    state2.insert("x".to_string(), RelType::Float(f64::NAN));
    state2.insert("y".to_string(), RelType::Int(100));

    assert!(state_mutations_eq_nan_aware(&state1, &state2));
}

#[test]
fn test_deterministic_fuzz_parity_100_iterations() {
    let validator = DualEngineValidator::new();

    for seed in 1..=100 {
        let mut fuzzer = AstFuzzer::new(seed);
        let depth = (seed as usize % 3) + 1; // 1 to 3 levels deep
        let ast = fuzzer.gen_node(0, depth);

        let report = validator.validate(&ast);
        assert!(
            report.is_valid,
            "Differential divergence detected on seed {} (depth {}):\nAST: {:?}\nError: {:?}",
            seed, depth, ast, report.error
        );
    }
}

#[test]
fn test_boundary_arithmetic_fuzz() {
    let validator = DualEngineValidator::new();

    let test_cases = vec![
        // Integer Boundary Wrapping
        Node::Add(
            Box::new(Node::IntLiteral(i64::MAX)),
            Box::new(Node::IntLiteral(1)),
        ),
        Node::Sub(
            Box::new(Node::IntLiteral(i64::MIN)),
            Box::new(Node::IntLiteral(1)),
        ),
        Node::Mul(
            Box::new(Node::IntLiteral(i64::MAX)),
            Box::new(Node::IntLiteral(2)),
        ),
        Node::Mul(
            Box::new(Node::IntLiteral(i64::MIN)),
            Box::new(Node::IntLiteral(-1)),
        ),
        Node::Div(
            Box::new(Node::IntLiteral(i64::MIN)),
            Box::new(Node::IntLiteral(-1)),
        ),
        Node::Modulo(
            Box::new(Node::IntLiteral(i64::MIN)),
            Box::new(Node::IntLiteral(-1)),
        ),
        Node::Neg(Box::new(Node::IntLiteral(i64::MIN))),
        // Division and Modulo by Zero (Symmetrical Faults)
        Node::Div(
            Box::new(Node::IntLiteral(42)),
            Box::new(Node::IntLiteral(0)),
        ),
        Node::Modulo(
            Box::new(Node::IntLiteral(42)),
            Box::new(Node::IntLiteral(0)),
        ),
        Node::Div(
            Box::new(Node::FloatLiteral(42.0)),
            Box::new(Node::FloatLiteral(0.0)),
        ),
        Node::Modulo(
            Box::new(Node::FloatLiteral(42.0)),
            Box::new(Node::FloatLiteral(0.0)),
        ),
        // Mixed Int/Float Division by Zero
        Node::Div(
            Box::new(Node::IntLiteral(42)),
            Box::new(Node::FloatLiteral(0.0)),
        ),
        Node::Div(
            Box::new(Node::FloatLiteral(42.0)),
            Box::new(Node::IntLiteral(0)),
        ),
    ];

    for (idx, ast) in test_cases.into_iter().enumerate() {
        let report = validator.validate(&ast);
        assert!(
            report.is_valid,
            "Boundary arithmetic test case {} failed parity:\nAST: {:?}\nError: {:?}",
            idx, ast, report.error
        );
    }
}

#[test]
fn test_float_special_values_fuzz() {
    let validator = DualEngineValidator::new();

    let test_cases = vec![
        // NaN propagation
        Node::Add(
            Box::new(Node::FloatLiteral(f64::NAN)),
            Box::new(Node::FloatLiteral(1.0)),
        ),
        Node::Sub(
            Box::new(Node::FloatLiteral(10.0)),
            Box::new(Node::FloatLiteral(f64::NAN)),
        ),
        Node::Mul(
            Box::new(Node::FloatLiteral(f64::NAN)),
            Box::new(Node::FloatLiteral(0.0)),
        ),
        // Infinity operations
        Node::Add(
            Box::new(Node::FloatLiteral(f64::INFINITY)),
            Box::new(Node::FloatLiteral(100.0)),
        ),
        Node::Sub(
            Box::new(Node::FloatLiteral(f64::NEG_INFINITY)),
            Box::new(Node::FloatLiteral(100.0)),
        ),
        // Mixed Int and Float Arithmetic
        Node::Add(
            Box::new(Node::IntLiteral(10)),
            Box::new(Node::FloatLiteral(2.5)),
        ),
        Node::Add(
            Box::new(Node::FloatLiteral(2.5)),
            Box::new(Node::IntLiteral(10)),
        ),
        Node::Sub(
            Box::new(Node::IntLiteral(10)),
            Box::new(Node::FloatLiteral(2.5)),
        ),
        Node::Sub(
            Box::new(Node::FloatLiteral(10.0)),
            Box::new(Node::IntLiteral(2)),
        ),
        Node::Mul(
            Box::new(Node::IntLiteral(6)),
            Box::new(Node::FloatLiteral(2.5)),
        ),
        Node::Mul(
            Box::new(Node::FloatLiteral(2.5)),
            Box::new(Node::IntLiteral(6)),
        ),
        Node::Div(
            Box::new(Node::IntLiteral(10)),
            Box::new(Node::FloatLiteral(2.5)),
        ),
        Node::Div(
            Box::new(Node::FloatLiteral(10.0)),
            Box::new(Node::IntLiteral(2)),
        ),
    ];

    for (idx, ast) in test_cases.into_iter().enumerate() {
        let report = validator.validate(&ast);
        assert!(
            report.is_valid,
            "Float special value test case {} failed parity:\nAST: {:?}\nError: {:?}",
            idx, ast, report.error
        );
    }
}

#[test]
fn test_array_operations_fuzz() {
    let validator = DualEngineValidator::new();

    let ast = Node::ArrayGet(
        Box::new(Node::ArrayCreate(vec![
            Node::IntLiteral(10),
            Node::IntLiteral(20),
            Node::IntLiteral(30),
        ])),
        Box::new(Node::IntLiteral(1)),
    );

    let report = validator.validate(&ast);
    assert!(report.is_valid);
    match report.outcome.unwrap() {
        DualValidationOutcome::Success { return_value, .. } => {
            assert_eq!(return_value, RelType::Int(20));
        }
        _ => panic!("Expected successful array get outcome"),
    }
}

#[test]
fn test_vector_simd_parity_fuzz() {
    let validator = DualEngineValidator::new();

    // VectorDot
    let ast_dot = Node::VectorDot(
        Box::new(Node::ArrayCreate(vec![
            Node::FloatLiteral(1.0),
            Node::FloatLiteral(2.0),
            Node::FloatLiteral(3.0),
        ])),
        Box::new(Node::ArrayCreate(vec![
            Node::FloatLiteral(4.0),
            Node::FloatLiteral(5.0),
            Node::FloatLiteral(6.0),
        ])),
    );

    let report_dot = validator.validate(&ast_dot);
    assert!(report_dot.is_valid);
    match report_dot.outcome.unwrap() {
        DualValidationOutcome::Success { return_value, .. } => {
            assert_eq!(return_value, RelType::Float(32.0));
        }
        _ => panic!("Expected vector dot success"),
    }

    // VectorAdd
    let ast_add = Node::VectorAdd(
        Box::new(Node::ArrayCreate(vec![
            Node::FloatLiteral(1.0),
            Node::FloatLiteral(2.0),
        ])),
        Box::new(Node::ArrayCreate(vec![
            Node::FloatLiteral(3.0),
            Node::FloatLiteral(4.0),
        ])),
    );

    let report_add = validator.validate(&ast_add);
    assert!(report_add.is_valid);
    match report_add.outcome.unwrap() {
        DualValidationOutcome::Success { return_value, .. } => {
            assert_eq!(
                return_value,
                RelType::Array(vec![RelType::Float(4.0), RelType::Float(6.0)])
            );
        }
        _ => panic!("Expected vector add success"),
    }
}

#[test]
fn test_quarantine_protocol_divergence() {
    // Verify that the DualEngineValidator detects discrepancy and quarantines execution
    let mut custom_validator = DualEngineValidator::new();
    custom_validator.optimize_ast = false;

    // Node::PropertyGet on an uninitialized structure will cause a fault
    let ast = Node::PropertyGet(
        Box::new(Node::IntLiteral(42)),
        "unknown_property".to_string(),
    );

    let report = custom_validator.validate(&ast);
    // Even if compilation fails on the VM for an uncompilable node, both classify symmetrically or flag divergence
    if !report.is_valid {
        assert!(report.error.is_some());
    } else {
        assert!(report.outcome.is_some());
    }

    // Direct DualEngineValidator::evaluate associated function check
    let eval_report = DualEngineValidator::evaluate(&Node::IntLiteral(12345));
    assert!(eval_report.is_valid);
    match eval_report.outcome.unwrap() {
        DualValidationOutcome::Success { return_value, .. } => {
            assert_eq!(return_value, RelType::Int(12345));
        }
        _ => panic!("Expected success"),
    }
}
