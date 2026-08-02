use crate::executor::{AgentPermissions, ExecResult, RelType};

pub mod bridge;
pub mod ffi_safety;
pub mod fs;
pub mod io;
pub mod math;
pub mod registry;

#[cfg(feature = "ui")]
pub mod geometry;
#[cfg(feature = "ui")]
pub mod physics;
#[cfg(feature = "ui")]
pub mod scene;
#[cfg(feature = "ui")]
pub mod ui;

pub trait NativeModule: Send {
    fn handle(
        &self,
        func_name: &str,
        args: &[RelType],
        permissions: &AgentPermissions,
    ) -> Option<ExecResult>;
}
