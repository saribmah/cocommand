//! Workspace configuration and session metadata.

pub mod config;
pub mod instance;
pub use config::{
    load_or_create_workspace_config, migrate_workspace_config, workspace_config_path,
    WorkspaceAiPreferences, WorkspaceApp, WorkspaceApps, WorkspaceConfig,
    WorkspacePreferences, WorkspaceTheme, ExtensionCachePreferences, SessionPreferences,
    WORKSPACE_CONFIG_FILENAME, WORKSPACE_CONFIG_VERSION,
};
pub use instance::WorkspaceInstance;
