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
}
