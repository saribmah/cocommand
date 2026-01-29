use std::path::{Path, PathBuf};

use crate::error::{CoreError, CoreResult};
use crate::workspace::config::{load_or_create_workspace_config, WorkspaceConfig};

#[derive(Debug, Clone)]
pub struct WorkspaceInstance {
    pub workspace_dir: PathBuf,
    pub config: WorkspaceConfig,
}

impl WorkspaceInstance {
    pub fn load(workspace_dir: &Path) -> CoreResult<Self> {
        if !workspace_dir.exists() {
            std::fs::create_dir_all(workspace_dir).map_err(|error| {
                CoreError::Internal(format!(
                    "failed to create workspace directory {}: {error}",
                    workspace_dir.display()
                ))
            })?;
        }
        let config = load_or_create_workspace_config(workspace_dir)?;
        Ok(Self {
            workspace_dir: workspace_dir.to_path_buf(),
            config,
        })
    }

    pub fn sessions_path(&self) -> PathBuf {
        self.workspace_dir.join("sessions.json")
    }
}
