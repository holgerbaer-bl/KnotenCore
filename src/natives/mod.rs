use crate::executor::{AgentPermissions, ExecResult, RelType};

pub mod bridge;
pub mod ffi_safety;
pub mod fs;
pub mod geometry;
pub mod io;
pub mod math;
pub mod physics;
pub mod registry;
pub mod scene;
pub mod ui;

pub trait NativeModule: Send {
    fn handle(
        &self,
        func_name: &str,
        args: &[RelType],
        permissions: &AgentPermissions,
    ) -> Option<ExecResult>;
}
