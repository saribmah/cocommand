use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};
use crate::storage::Storage;
use crate::utils::time::now_secs;

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
    pub ai: WorkspaceAiPreferences,
    pub theme: WorkspaceTheme,
    pub onboarding: WorkspaceOnboarding,
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
    pub application_cache: ApplicationCachePreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceAiPreferences {
    pub provider: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub system_prompt: String,
    pub temperature: f64,
    pub max_output_tokens: u32,
    pub max_steps: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPreferences {
    pub rollover_mode: String,
    pub duration_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationCachePreferences {
    pub max_applications: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceTheme {
    pub mode: String,
    pub accent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceOnboarding {
    pub completed: bool,
    pub completed_at: Option<u64>,
    pub version: String,
}

impl WorkspaceConfig {
    pub fn default_new() -> Self {
        let now = now_secs();
        Self {
            version: WORKSPACE_CONFIG_VERSION.to_string(),
            workspace_id: Uuid::now_v7().to_string(),
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
                application_cache: ApplicationCachePreferences {
                    max_applications: 8,
                },
            },
            ai: WorkspaceAiPreferences {
                provider: "openai-compatible".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: None,
                model: "gpt-4o-mini".to_string(),
                system_prompt: "You are Cocommand, a helpful command assistant.".to_string(),
                temperature: 0.7,
                max_output_tokens: 80_000,
                max_steps: 8,
            },
            theme: WorkspaceTheme {
                mode: "system".to_string(),
                accent: "copper".to_string(),
            },
            onboarding: WorkspaceOnboarding {
                completed: false,
                completed_at: None,
                version: "1.0".to_string(),
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

pub async fn load_or_create_workspace_storage(
    storage: &dyn Storage,
) -> CoreResult<WorkspaceConfig> {
    let mut workspace_ids = storage.list(&["workspace"]).await?;
    if workspace_ids.is_empty() {
        let config = WorkspaceConfig::default_new();
        let value = serde_json::to_value(&config).map_err(|error| {
            CoreError::Internal(format!("failed to serialize workspace config: {error}"))
        })?;
        storage
            .write(&["workspace", &config.workspace_id], &value)
            .await?;
        return Ok(config);
    }
    workspace_ids.sort();
    let last_id = workspace_ids.last().cloned().unwrap();
    let value = storage
        .read(&["workspace", &last_id])
        .await?
        .ok_or_else(|| CoreError::Internal("workspace config missing".to_string()))?;
    let mut config: WorkspaceConfig = serde_json::from_value(value).map_err(|error| {
        CoreError::Internal(format!("failed to parse workspace config: {error}"))
    })?;
    if config.version != WORKSPACE_CONFIG_VERSION {
        config = migrate_workspace_config(config)?;
        let value = serde_json::to_value(&config).map_err(|error| {
            CoreError::Internal(format!("failed to serialize workspace config: {error}"))
        })?;
        storage
            .write(&["workspace", &config.workspace_id], &value)
            .await?;
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
