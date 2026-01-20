//! window.restore_workspace tool implementation.
//!
//! Restores a soft-reset or archived workspace to active state.

use std::sync::Arc;

use llm_kit_provider_utils::tool::{Tool, ToolExecutionOutput};
use serde_json::json;

use crate::storage::WorkspaceStore;
use crate::workspace::service::WorkspaceService;

/// Tool ID for the restore_workspace tool
pub const TOOL_ID: &str = "window_restore_workspace";

/// Build the window.restore_workspace tool.
///
/// This tool restores a soft-reset or archived workspace:
/// - Reactivates the workspace by updating last_active_at
/// - Resets staleness level to "fresh"
/// - Returns the restored workspace snapshot with all previously open apps
pub fn build(store: Arc<dyn WorkspaceStore>, workspace: WorkspaceService) -> (String, Tool) {
    let tool = Tool::function(json!({
        "type": "object",
        "properties": {},
        "required": []
    }))
    .with_description(
        "Restore a soft-reset or archived workspace. Re-activates the workspace and returns the full snapshot with all previously open apps.",
    )
    .with_execute(Arc::new(move |_input, _opts| {
        let store = store.clone();
        let workspace = workspace.clone();

        ToolExecutionOutput::Single(Box::pin(async move {
            match store.load() {
                Ok(mut workspace_state) => {
                    // Touch the workspace to reset last_active_at and staleness
                    workspace.touch(&mut workspace_state);

                    match store.save(&workspace_state) {
                        Ok(()) => {
                            let snapshot = workspace.snapshot(&workspace_state);
                            Ok(json!({
                                "status": "restored",
                                "message": "Workspace has been restored to active state",
                                "snapshot": snapshot
                            }))
                        }
                        Err(error) => Err(json!({ "error": error })),
                    }
                }
                Err(error) => Err(json!({ "error": error })),
            }
        }))
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
        assert_eq!(id, "window_restore_workspace");
    }

    #[test]
    fn test_tool_has_description() {
        let store: Arc<dyn WorkspaceStore> = Arc::new(MemoryStore::default());
        let workspace = WorkspaceService::new();
        let (_id, tool) = build(store, workspace);
        assert!(tool.description.is_some());
        assert!(tool.description.as_ref().unwrap().contains("Restore"));
    }

    #[test]
    fn test_tool_schema_has_no_required_params() {
        let store: Arc<dyn WorkspaceStore> = Arc::new(MemoryStore::default());
        let workspace = WorkspaceService::new();
        let (_id, tool) = build(store, workspace);
        let schema = &tool.input_schema;
        let required = schema.get("required").and_then(|r| r.as_array());
        assert!(required.is_some());
        assert!(required.unwrap().is_empty());
    }
}
