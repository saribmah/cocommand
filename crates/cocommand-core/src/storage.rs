use crate::workspace::types::WorkspaceState;

pub mod file;
pub mod memory;

pub trait WorkspaceStore: Send + Sync {
    fn load(&self) -> Result<WorkspaceState, String>;
    fn save(&self, state: &WorkspaceState) -> Result<(), String>;
}
