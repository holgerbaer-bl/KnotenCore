#[derive(Debug, Clone, PartialEq)]
pub enum OpCode {
    Constant(usize),
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    Greater,
    Less,
    Jump(usize),
    JumpIfFalse(usize),
    StringLength,
    StringContainsChars,
    SetGlobal(usize),
    GetGlobal(usize),
    Print,
    Return,
}
