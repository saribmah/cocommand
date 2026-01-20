//! window.focus tool implementation.
//!
//! Focuses an already-open application in the workspace.
//! Respects workspace lifecycle - blocked when workspace is archived.

use std::sync::Arc;

use llm_kit_provider_utils::tool::{Tool, ToolExecutionOutput};
use serde_json::json;

use crate::storage::WorkspaceStore;
use crate::workspace::service::WorkspaceService;

/// Tool ID for the focus tool
pub const TOOL_ID: &str = "window_focus";

/// Build the window.focus tool.
///
/// This tool sets focus to an already-open application. It will return
/// an error if the specified app is not currently open.
///
/// Note: This tool is blocked when the workspace is archived.
/// Use window.restore_workspace to recover before focusing apps.
pub fn build(store: Arc<dyn WorkspaceStore>, workspace: WorkspaceService) -> (String, Tool) {
    let tool = Tool::function(json!({
        "type": "object",
        "properties": {
            "appId": {
                "type": "string",
                "description": "The ID of the application to focus"
            }
        },
        "required": ["appId"]
    }))
    .with_description("Focus an already-open application in the workspace.")
    .with_execute(Arc::new(move |input, _opts| {
        let app_id = input
            .get("appId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let result = if app_id.is_empty() {
            Err(json!({ "error": "appId is required" }))
        } else {
            match store.load() {
                Ok(mut workspace_state) => {
                    // Block focus operations on archived workspaces
                    if workspace.is_archived(&workspace_state) {
                        return ToolExecutionOutput::Single(Box::pin(async move {
                            Err(json!({
                                "error": "workspace_archived",
                                "message": "Workspace is archived. Use window.restore_workspace to recover."
                            }))
                        }));
                    }

                    // Check if app is open before focusing
                    let is_open = workspace_state.open_apps.iter().any(|app| app.id == app_id);

                    if !is_open {
                        Err(json!({
                            "error": "app_not_open",
                            "appId": app_id,
                            "hint": "Use window.open to open the app first"
                        }))
                    } else {
                        workspace.focus_app(&mut workspace_state, &app_id);
                        match store.save(&workspace_state) {
                            Ok(()) => {
                                let snapshot = workspace.snapshot(&workspace_state);
                                Ok(json!({
                                    "status": "focused",
                                    "appId": app_id,
                                    "snapshot": snapshot
                                }))
                            }
                            Err(error) => Err(json!({ "error": error })),
                        }
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
        assert_eq!(id, "window_focus");
    }

    #[test]
    fn test_tool_has_description() {
        let store: Arc<dyn WorkspaceStore> = Arc::new(MemoryStore::default());
        let workspace = WorkspaceService::new();
        let (_id, tool) = build(store, workspace);
        assert!(tool.description.is_some());
        assert!(tool.description.as_ref().unwrap().contains("Focus"));
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
