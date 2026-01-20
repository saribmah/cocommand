use std::sync::Arc;

use llm_kit_core::ToolSet;
use llm_kit_provider_utils::tool::{Tool, ToolExecutionOutput};
use serde_json::json;

use crate::applications;
use crate::storage::WorkspaceStore;
use crate::workspace::service::WorkspaceService;

/// Build the window management tools that form the control plane
/// These tools are always available regardless of which apps are open
pub fn build_window_tools(
    store: Arc<dyn WorkspaceStore>,
    workspace: WorkspaceService,
) -> ToolSet {
    let mut tools = ToolSet::new();

    // window.list_apps - List all available applications
    tools.insert(
        "window.list_apps".to_string(),
        Tool::function(json!({
            "type": "object",
            "properties": {},
            "required": []
        }))
        .with_description("List all available applications that can be opened in the workspace.")
        .with_execute(Arc::new(move |_input, _opts| {
            let apps: Vec<_> = applications::all_apps()
                .into_iter()
                .map(|app| {
                    json!({
                        "id": app.id,
                        "name": app.name,
                        "description": app.description
                    })
                })
                .collect();
            ToolExecutionOutput::Single(Box::pin(async move { Ok(json!(apps)) }))
        })),
    );

    // window.get_snapshot - Get current workspace state
    let snapshot_store = store.clone();
    let snapshot_workspace = workspace.clone();
    tools.insert(
        "window.get_snapshot".to_string(),
        Tool::function(json!({
            "type": "object",
            "properties": {},
            "required": []
        }))
        .with_description("Get the current workspace snapshot showing focused app, open apps, and staleness.")
        .with_execute(Arc::new(move |_input, _opts| {
            let result = snapshot_store
                .load()
                .map(|workspace_state| snapshot_workspace.snapshot(&workspace_state))
                .map(|snapshot| json!(snapshot))
                .map_err(|error| json!({ "error": error }));
            ToolExecutionOutput::Single(Box::pin(async move { result }))
        })),
    );

    // window.open - Open an application and mount its tools
    let open_store = store.clone();
    let open_workspace = workspace.clone();
    tools.insert(
        "window.open".to_string(),
        Tool::function(json!({
            "type": "object",
            "properties": {
                "appId": {
                    "type": "string",
                    "description": "The ID of the application to open (e.g., 'spotify')"
                }
            },
            "required": ["appId"]
        }))
        .with_description("Open an application in the workspace. This mounts the app's tools and sets focus to it.")
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
                match open_store.load() {
                    Ok(mut workspace_state) => {
                        open_workspace.open_app(&mut workspace_state, &app_id);
                        match open_store.save(&workspace_state) {
                            Ok(()) => {
                                let snapshot = open_workspace.snapshot(&workspace_state);
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
        })),
    );

    // window.close - Close an application and unmount its tools
    let close_store = store.clone();
    let close_workspace = workspace.clone();
    tools.insert(
        "window.close".to_string(),
        Tool::function(json!({
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
                match close_store.load() {
                    Ok(mut workspace_state) => {
                        close_workspace.close_app(&mut workspace_state, &app_id);
                        match close_store.save(&workspace_state) {
                            Ok(()) => {
                                let snapshot = close_workspace.snapshot(&workspace_state);
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
        })),
    );

    // window.focus - Focus an already-open application
    let focus_store = store.clone();
    let focus_workspace = workspace.clone();
    tools.insert(
        "window.focus".to_string(),
        Tool::function(json!({
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
                match focus_store.load() {
                    Ok(mut workspace_state) => {
                        // Check if app is open before focusing
                        let is_open = workspace_state
                            .open_apps
                            .iter()
                            .any(|app| app.id == app_id);

                        if !is_open {
                            Err(json!({
                                "error": "app_not_open",
                                "appId": app_id,
                                "hint": "Use window.open to open the app first"
                            }))
                        } else {
                            focus_workspace.focus_app(&mut workspace_state, &app_id);
                            match focus_store.save(&workspace_state) {
                                Ok(()) => {
                                    let snapshot = focus_workspace.snapshot(&workspace_state);
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
        })),
    );

    tools
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory::MemoryStore;

    #[test]
    fn test_build_window_tools() {
        let store = Arc::new(MemoryStore::new());
        let workspace = WorkspaceService::new();
        let tools = build_window_tools(store, workspace);

        assert!(tools.get("window.list_apps").is_some());
        assert!(tools.get("window.get_snapshot").is_some());
        assert!(tools.get("window.open").is_some());
        assert!(tools.get("window.close").is_some());
        assert!(tools.get("window.focus").is_some());
    }
}
