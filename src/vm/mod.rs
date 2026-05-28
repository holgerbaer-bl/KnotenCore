pub mod compiler;
pub mod machine;
pub mod storage;

// Sprint 211: OpCode now from shared peer-crate
pub use knoten_core_types::opcode;
pub use knoten_core_types::opcode::OpCode;
pub use knoten_core_types::opcode::SimdOp;

pub use compiler::Compiler;
pub use machine::VM;
