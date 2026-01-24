use std::sync::{Arc, Mutex};

use cocommand::Core;
use cocommand::storage::MemoryStorage;

/// Shared application state holding the Core instance.
/// Wrapped in Arc<Mutex<_>> because Core::submit_command requires &mut self.
pub struct AppState {
    pub core: Arc<Mutex<Core>>,
}

impl AppState {
    pub fn new() -> Self {
        let storage = Box::new(MemoryStorage::new());
        Self {
            core: Arc::new(Mutex::new(Core::new(storage))),
        }
    }
}
