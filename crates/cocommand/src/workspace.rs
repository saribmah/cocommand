//! Workspace state, invariants, kernel tools, and atomic patch application (Core-1).

pub mod state;
pub mod invariants;
pub mod kernel_tools;
pub mod patch;
pub mod config;

pub use state::*;
pub use invariants::validate_invariants;
pub use kernel_tools::*;
pub use patch::{apply_patch, PatchResult, WorkspaceOp, WorkspacePatch};
pub use config::{
    load_or_create_workspace_config, migrate_workspace_config, workspace_config_path,
    WorkspaceApp, WorkspaceApps, WorkspaceConfig, WorkspacePreferences, WorkspaceTheme,
    WindowCachePreferences, SessionPreferences, WORKSPACE_CONFIG_FILENAME,
    WORKSPACE_CONFIG_VERSION,
};
