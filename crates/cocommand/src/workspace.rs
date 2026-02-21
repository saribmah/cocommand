//! Workspace configuration and session metadata.

pub mod config;
pub mod instance;
pub mod setup;
pub use config::{
    load_or_create_workspace_config, migrate_workspace_config, workspace_config_path,
    ExtensionCachePreferences, FileSystemPreferences, SessionPreferences, WorkspaceConfig,
    WorkspaceExtension, WorkspaceExtensions, WorkspaceLLMPreferences, WorkspacePreferences,
    WorkspaceTheme, WORKSPACE_CONFIG_FILENAME, WORKSPACE_CONFIG_VERSION,
};
pub use instance::WorkspaceInstance;
