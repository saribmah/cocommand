//! window.close tool implementation.
//!
//! Closes an application in the workspace and unmounts its tools.
//! Respects workspace lifecycle - blocked when workspace is archived.

use std::sync::Arc;

use llm_kit_provider_utils::tool::{Tool, ToolExecutionOutput};
use serde_json::json;

use crate::applications;
use crate::storage::WorkspaceStore;
use crate::workspace::service::WorkspaceService;

/// Tool ID for the close tool
pub const TOOL_ID: &str = "window_close";

/// Build the window.close tool.
///
/// This tool closes an application in the workspace, which:
/// - Removes the app from the open apps list
/// - Unmounts the app's tools
/// - Shifts focus to the last remaining open app (if any)
///
/// Note: This tool is blocked when the workspace is archived.
/// Use window.restore_workspace to recover before closing apps.
pub fn build(store: Arc<dyn WorkspaceStore>, workspace: WorkspaceService) -> (String, Tool) {
    let tool = Tool::function(json!({
        "type": "object",
        "properties": {
            "appId": {
                "type": "string",
                "description": "The ID of the application to close"
            }
        },
        "required": ["appId"]
    }))
    .with_description("Close an application in the workspace. This unmounts the app's tools.")
    .with_execute(Arc::new(move |input, _opts| {
        let app_id = input
            .get("appId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let result = if app_id.is_empty() {
            Err(json!({ "error": "appId is required" }))
        } else if applications::app_by_id(&app_id).is_none() {
            Err(json!({ "error": "unknown_app", "appId": app_id }))
        } else {
            match store.load() {
                Ok(mut workspace_state) => {
                    // Block close operations on archived workspaces
                    if workspace.is_archived(&workspace_state) {
                        return ToolExecutionOutput::Single(Box::pin(async move {
                            Err(json!({
                                "error": "workspace_archived",
                                "message": "Workspace is archived. Use window.restore_workspace to recover."
                            }))
                        }));
                    }

                    workspace.close_app(&mut workspace_state, &app_id);
                    match store.save(&workspace_state) {
                        Ok(()) => {
                            let snapshot = workspace.snapshot(&workspace_state);
                            Ok(json!({
                                "status": "closed",
                                "appId": app_id,
                                "snapshot": snapshot
                            }))
                        }
                        Err(error) => Err(json!({ "error": error })),
                    }
                }
                Err(error) => Err(json!({ "error": error })),
            }
        };
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
        assert_eq!(id, "window_close");
    }

    #[test]
    fn test_tool_has_description() {
        let store: Arc<dyn WorkspaceStore> = Arc::new(MemoryStore::default());
        let workspace = WorkspaceService::new();
        let (_id, tool) = build(store, workspace);
        assert!(tool.description.is_some());
        assert!(tool.description.as_ref().unwrap().contains("Close"));
    }

    #[test]
    fn test_tool_schema_requires_app_id() {
        let store: Arc<dyn WorkspaceStore> = Arc::new(MemoryStore::default());
        let workspace = WorkspaceService::new();
        let (_id, tool) = build(store, workspace);
        let schema = &tool.input_schema;
        let required = schema.get("required").and_then(|r| r.as_array());
        assert!(required.is_some());
        assert!(required
            .unwrap()
            .iter()
            .any(|v| v.as_str() == Some("appId")));
    }
}
