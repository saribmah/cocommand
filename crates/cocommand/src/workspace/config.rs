use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};

pub const WORKSPACE_CONFIG_FILENAME: &str = "workspace.json";
pub const WORKSPACE_CONFIG_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub version: String,
    pub workspace_id: String,
    pub name: String,
    pub created_at: u64,
    pub last_modified: u64,
    pub apps: WorkspaceApps,
    pub preferences: WorkspacePreferences,
    pub theme: WorkspaceTheme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceApps {
    pub installed: Vec<WorkspaceApp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceApp {
    pub app_id: String,
    pub version: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacePreferences {
    pub language: String,
    pub session: SessionPreferences,
    pub window_cache: WindowCachePreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPreferences {
    pub rollover_mode: String,
    pub duration_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowCachePreferences {
    pub max_windows: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceTheme {
    pub mode: String,
    pub accent: String,
}

impl WorkspaceConfig {
    pub fn default_new() -> Self {
        let now = now_secs();
        Self {
            version: WORKSPACE_CONFIG_VERSION.to_string(),
            workspace_id: Uuid::new_v4().to_string(),
            name: "Default Workspace".to_string(),
            created_at: now,
            last_modified: now,
            apps: WorkspaceApps { installed: Vec::new() },
            preferences: WorkspacePreferences {
                language: "en".to_string(),
                session: SessionPreferences {
                    rollover_mode: "rolling".to_string(),
                    duration_seconds: 86_400,
                },
                window_cache: WindowCachePreferences { max_windows: 8 },
            },
            theme: WorkspaceTheme {
                mode: "system".to_string(),
                accent: "blue".to_string(),
            },
        }
    }
}

pub fn load_or_create_workspace_config(dir: &Path) -> CoreResult<WorkspaceConfig> {
    std::fs::create_dir_all(dir).map_err(|error| {
        CoreError::Internal(format!(
            "failed to create workspace directory {}: {error}",
            dir.display()
        ))
    })?;

    let path = dir.join(WORKSPACE_CONFIG_FILENAME);
    if !path.exists() {
        let config = WorkspaceConfig::default_new();
        write_workspace_config(&path, &config)?;
        return Ok(config);
    }

    let data = std::fs::read_to_string(&path).map_err(|error| {
        CoreError::Internal(format!(
            "failed to read workspace config {}: {error}",
            path.display()
        ))
    })?;
    let mut config: WorkspaceConfig = serde_json::from_str(&data).map_err(|error| {
        CoreError::Internal(format!(
            "failed to parse workspace config {}: {error}",
            path.display()
        ))
    })?;

    if config.version != WORKSPACE_CONFIG_VERSION {
        config = migrate_workspace_config(config)?;
        write_workspace_config(&path, &config)?;
    }

    Ok(config)
}

pub fn migrate_workspace_config(_config: WorkspaceConfig) -> CoreResult<WorkspaceConfig> {
    Err(CoreError::NotImplemented)
}

pub fn workspace_config_path(dir: &Path) -> PathBuf {
    dir.join(WORKSPACE_CONFIG_FILENAME)
}

fn write_workspace_config(path: &Path, config: &WorkspaceConfig) -> CoreResult<()> {
    let data = serde_json::to_string_pretty(config).map_err(|error| {
        CoreError::Internal(format!(
            "failed to serialize workspace config {}: {error}",
            path.display()
        ))
    })?;
    std::fs::write(path, data).map_err(|error| {
        CoreError::Internal(format!(
            "failed to write workspace config {}: {error}",
            path.display()
        ))
    })?;
    Ok(())
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CoreError;
    use tempfile::tempdir;

    #[test]
    fn creates_config_when_missing() {
        let dir = tempdir().expect("tempdir");
        let config = load_or_create_workspace_config(dir.path()).expect("load/create");

        let path = workspace_config_path(dir.path());
        assert!(path.exists());
        assert_eq!(config.version, WORKSPACE_CONFIG_VERSION);
        assert_eq!(config.name, "Default Workspace");
    }

    #[test]
    fn loads_existing_config() {
        let dir = tempdir().expect("tempdir");
        let original = WorkspaceConfig::default_new();
        let path = workspace_config_path(dir.path());
        write_workspace_config(&path, &original).expect("write config");

        let loaded = load_or_create_workspace_config(dir.path()).expect("load config");
        assert_eq!(loaded.workspace_id, original.workspace_id);
    }

    #[test]
    fn version_mismatch_invokes_migration_stub() {
        let dir = tempdir().expect("tempdir");
        let mut original = WorkspaceConfig::default_new();
        original.version = "0.9.0".to_string();
        let path = workspace_config_path(dir.path());
        write_workspace_config(&path, &original).expect("write config");

        let err = load_or_create_workspace_config(dir.path()).expect_err("expected error");
        match err {
            CoreError::NotImplemented => {}
            other => panic!("expected NotImplemented, got {other:?}"),
        }
    }
}
