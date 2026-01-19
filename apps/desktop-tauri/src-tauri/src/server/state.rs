use std::sync::Arc;

use crate::storage::WorkspaceStore;
use crate::workspace::service::WorkspaceService;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn WorkspaceStore>,
    pub workspace: WorkspaceService,
}

impl AppState {
    pub fn new(store: Arc<dyn WorkspaceStore>) -> Self {
        AppState {
            store,
            workspace: WorkspaceService::new(),
        }
    }
}
