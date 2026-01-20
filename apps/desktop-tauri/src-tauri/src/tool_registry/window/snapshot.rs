//! window.get_snapshot tool implementation.
//!
//! Gets the current workspace snapshot showing focused app, open apps, and staleness.
//! Respects workspace lifecycle rules (soft reset/archived).

use std::sync::Arc;

use llm_kit_provider_utils::tool::{Tool, ToolExecutionOutput};
use serde_json::json;

use crate::storage::WorkspaceStore;
use crate::workspace::service::WorkspaceService;

/// Tool ID for the get_snapshot tool
pub const TOOL_ID: &str = "window.get_snapshot";

/// Build the window.get_snapshot tool.
///
/// This tool returns the current workspace snapshot showing:
/// - The focused application
/// - All open applications
/// - Staleness level
///
/// The snapshot respects workspace lifecycle rules:
/// - Fresh/Stale: Returns full snapshot
/// - Dormant (24h-7d): Returns soft-reset snapshot (empty apps, offer restore)
/// - Archived (>7d): Returns archived status, requires explicit restore
pub fn build(store: Arc<dyn WorkspaceStore>, workspace: WorkspaceService) -> (String, Tool) {
    let tool = Tool::function(json!({
        "type": "object",
        "properties": {},
        "required": []
    }))
    .with_description(
        "Get the current workspace snapshot showing focused app, open apps, and staleness.",
    )
    .with_execute(Arc::new(move |_input, _opts| {
        let result = store
            .load()
            .map(|workspace_state| {
                // Apply lifecycle rules when building snapshot
                if workspace.is_archived(&workspace_state) {
                    // Archived workspaces require restore
                    crate::workspace::types::WorkspaceSnapshot {
                        focused_app: None,
                        open_apps: vec![],
                        staleness: "archived".to_string(),
                    }
                } else if workspace.should_soft_reset(&workspace_state) {
                    // Soft-reset: hide apps, offer restore
                    workspace.soft_reset_snapshot(&workspace_state)
                } else {
                    // Fresh or stale: return normal snapshot
                    workspace.snapshot(&workspace_state)
                }
            })
            .map(|snapshot| json!(snapshot))
            .map_err(|error| json!({ "error": error }));
        ToolExecutionOutput::Single(Box::pin(async move { result }))
    }));

    (TOOL_ID.to_string(), tool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory::MemoryStore;

    #[test]
    fn test_tool_id() {
        let store: Arc<dyn WorkspaceStore> = Arc::new(MemoryStore::default());
        let workspace = WorkspaceService::new();
        let (id, _tool) = build(store, workspace);
        assert_eq!(id, "window.get_snapshot");
    }

    #[test]
    fn test_tool_has_description() {
        let store: Arc<dyn WorkspaceStore> = Arc::new(MemoryStore::default());
        let workspace = WorkspaceService::new();
        let (_id, tool) = build(store, workspace);
        assert!(tool.description.is_some());
        assert!(tool.description.as_ref().unwrap().contains("snapshot"));
    }
}
