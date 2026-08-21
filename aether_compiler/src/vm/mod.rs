pub mod compiler;
pub mod dual_validator;
pub mod gpgpu;
pub mod inspector;
pub mod isolate;
pub mod machine;
pub mod native_emit;
pub mod scheduler;
pub mod shader_graph;
pub mod snapshot;
pub mod storage;
pub mod vfs; // Sprint 306: Sandboxed In-Memory VFS

pub use compiler::Compiler;
pub use dual_validator::{
    DualEngineValidator, DualValidationError, DualValidationOutcome, DualValidationReport,
    FaultCategory, classify_fault,
};
pub use knoten_core_types::opcode;
pub use machine::*;
// Sprint 303: re-export new module APIs
pub use gpgpu::{apply_matrix_to_inputs, split_inputs_to_bindings};
pub use inspector::{
    VMInspectorData, drain_hot_path_table, get_ledger_nonce, get_vm_inspection_snapshot,
    verify_ledger_hash,
};
// Sprint 306: re-export VFS
pub use vfs::VirtualFs;
