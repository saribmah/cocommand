use std::sync::Mutex;

use crate::storage::WorkspaceStore;
use crate::workspace::types::WorkspaceState;

pub struct MemoryStore {
    state: Mutex<WorkspaceState>,
}

impl MemoryStore {
    pub fn new(state: WorkspaceState) -> Self {
        MemoryStore {
            state: Mutex::new(state),
        }
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        MemoryStore::new(WorkspaceState::default())
    }
}

impl WorkspaceStore for MemoryStore {
    fn load(&self) -> Result<WorkspaceState, String> {
        self.state
            .lock()
            .map(|state| state.clone())
            .map_err(|_| "Failed to lock workspace state".to_string())
    }

    fn save(&self, state: &WorkspaceState) -> Result<(), String> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| "Failed to lock workspace state".to_string())?;
        *guard = state.clone();
        Ok(())
    }
}
