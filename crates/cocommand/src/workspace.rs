//! Workspace state, invariants, kernel tools, and atomic patch application (Core-1).

pub mod state;
pub mod invariants;
pub mod kernel_tools;
pub mod patch;

pub use state::*;
pub use invariants::validate_invariants;
pub use kernel_tools::*;
pub use patch::{apply_patch, PatchResult, WorkspaceOp, WorkspacePatch};
