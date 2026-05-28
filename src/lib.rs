pub mod async_bridge;
pub mod audio;
pub mod executor;
pub mod math;
pub mod natives;
pub mod test_lib;
pub mod window;

// Sprint 211: Types now served from shared peer-crate knoten_core_types
pub use knoten_core_types::ast;
pub use knoten_core_types::opcode;

pub mod compiler;
pub mod dsl_emitter;
pub mod evaluator;
pub mod optimizer;
pub mod parser;
pub mod validator;
pub mod vm;
