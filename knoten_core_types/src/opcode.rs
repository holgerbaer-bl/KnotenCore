// Sprint 202: SIMD operation types for auto-vectorization
#[derive(Debug, Clone, PartialEq)]
pub enum SimdOp {
    Scale,     // elements * factor
    Add,       // elements_a + elements_b
    Subtract,  // elements_a - elements_b
    Dot,       // elements_a · elements_b → scalar
    Transform, // Sprint 249: transform via registered matrix handle
}

#[derive(Debug, Clone, PartialEq)]
pub enum OpCode {
    Constant(usize),
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Neg,
    Equal,
    NotEqual,
    Greater,
    Less,
    LessEqual,
    GreaterEqual,
    And,
    Or,
    Not,
    Jump(usize),
    JumpIfFalse(usize),
    StringLength,
    StringContainsChars,
    StringSplit,
    ArrayContains,
    ReadFile,
    ExternCall {
        name_idx: usize,
        arg_count: usize,
    },
    SetGlobal(usize),
    GetGlobal(usize),
    SetLocal(usize),
    GetLocal(usize),
    Call(usize, usize),
    AllocateDict,
    GetProperty,
    SetProperty,
    Pop,
    Print,
    Return,

    // Sprint 127: Data Operations & String manipulation
    ArrayCreate(usize),
    ArrayGet,
    ArraySet,
    ArrayPush,
    ArrayLen,
    Concat,
    ToString,

    // Sprint 127: IO & System
    WriteFile,
    NativeExternCall {
        module_idx: usize,
        func_idx: usize,
        arg_count: usize,
    },

    // Sprint 127: UI Layouts & Rendering
    UIWindow(usize, usize), // (id_idx, children_count)
    UILabel,
    UIButton,
    UITextInput,
    UIHBox(usize),
    UIVBox(usize),

    LoadComputeShader,
    DispatchCompute(usize), // arg_count

    // Sprint 200/202: SIMD auto-vectorization — 4-element parallel ops
    SimdExec {
        op: SimdOp,
        elements_a: [usize; 4], // constant pool indices for the 4 float elements (first operand)
        elements_b: [usize; 4], // constant pool indices for second operand (Add/Sub/Dot)
        scale: usize,           // constant pool index for scale factor (Scale only)
        matrix_handle: i64,     // Sprint 249: matrix registry handle (Transform)
    },

    // Sprint 222: Neural DSL Synth — procedural audio note generation
    OpPlayNote, // pops frequency (f32) and duration_ms (i64) from stack
    OpStopNote, // pops channel (i64) from stack

    // Sprint 224: Continuous GPGPU compute streaming loop
    OpDispatchComputeLoop(usize), // arg_count
}
