//! window.open tool implementation.
//!
//! Opens an application in the workspace and mounts its tools.

use std::sync::Arc;

use llm_kit_provider_utils::tool::{Tool, ToolExecutionOutput};
use serde_json::json;

use crate::applications;
use crate::storage::WorkspaceStore;
use crate::workspace::service::WorkspaceService;

/// Tool ID for the open tool
pub const TOOL_ID: &str = "window_open";

/// Build the window.open tool.
///
/// This tool opens an application in the workspace, which:
/// - Adds the app to the open apps list
/// - Sets focus to the newly opened app
/// - Makes the app's tools available in execution phase
pub fn build(store: Arc<dyn WorkspaceStore>, workspace: WorkspaceService) -> (String, Tool) {
    let tool = Tool::function(json!({
        "type": "object",
        "properties": {
            "appId": {
                "type": "string",
                "description": "The ID of the application to open (e.g., 'spotify')"
            }
        },
        "required": ["appId"]
    }))
    .with_description(
        "Open an application in the workspace. This mounts the app's tools and sets focus to it.",
    )
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
                    // Block open operations on archived workspaces
                    if workspace.is_archived(&workspace_state) {
                        return ToolExecutionOutput::Single(Box::pin(async move {
                            Err(json!({
                                "error": "workspace_archived",
                                "message": "Workspace is archived. Use window.restore_workspace to recover."
                            }))
                        }));
                    }

                    workspace.open_app(&mut workspace_state, &app_id);
                    match store.save(&workspace_state) {
                        Ok(()) => {
                            let snapshot = workspace.snapshot(&workspace_state);
                            Ok(json!({
                                "status": "opened",
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
        assert_eq!(id, "window_open");
    }

    #[test]
    fn test_tool_has_description() {
        let store: Arc<dyn WorkspaceStore> = Arc::new(MemoryStore::default());
        let workspace = WorkspaceService::new();
        let (_id, tool) = build(store, workspace);
        assert!(tool.description.is_some());
        assert!(tool.description.as_ref().unwrap().contains("Open"));
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
