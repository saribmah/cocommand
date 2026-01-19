use std::fs;
use std::path::PathBuf;

use crate::storage::WorkspaceStore;
use crate::workspace::types::WorkspaceState;

pub struct FileStore {
    path: PathBuf,
}

impl FileStore {
    pub fn new(path: PathBuf) -> Self {
        FileStore { path }
    }
}

impl WorkspaceStore for FileStore {
    fn load(&self) -> Result<WorkspaceState, String> {
        if !self.path.exists() {
            return Ok(WorkspaceState::default());
        }
        let data = fs::read_to_string(&self.path).map_err(|error| error.to_string())?;
        serde_json::from_str(&data).map_err(|error| error.to_string())
    }

    fn save(&self, state: &WorkspaceState) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let data = serde_json::to_string_pretty(state).map_err(|error| error.to_string())?;
        fs::write(&self.path, data).map_err(|error| error.to_string())
    }
}
