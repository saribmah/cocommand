//! Tool registry for building tool sets.
//!
//! This module provides builders for different tool set configurations:
//! - Control plane: Only window.* tools (for workspace management)
//! - Execution plane: Window.* plus app-specific tools

use std::collections::HashSet;
use std::sync::Arc;

use llm_kit_core::ToolSet;
use llm_kit_provider_utils::tool::{Tool, ToolExecutionOutput};
use serde_json::json;

use crate::applications;
use crate::storage::WorkspaceStore;
use crate::workspace::service::WorkspaceService;
use crate::workspace::types::WorkspaceState;

use super::window::build_window_tools;

/// Build the control plane tool set.
///
/// Only includes window.* tools for workspace management.
/// This is used in the first phase of the agent loop.
pub fn build_control_plane_tool_set(
    store: Arc<dyn WorkspaceStore>,
    workspace: WorkspaceService,
) -> ToolSet {
    build_window_tools(store, workspace)
}

/// Build the execution plane tool set.
///
/// Includes window.* tools plus app tools for currently open apps only.
/// This is used after apps have been opened in the control phase.
pub fn build_execution_plane_tool_set(
    store: Arc<dyn WorkspaceStore>,
    workspace: WorkspaceService,
    workspace_state: &WorkspaceState,
) -> ToolSet {
    // Start with window tools
    let mut tools = build_window_tools(store.clone(), workspace);

    // Get the set of open app IDs
    let open_app_ids: HashSet<String> = workspace_state
        .open_apps
        .iter()
        .map(|app| app.id.clone())
        .collect();

    // Add tools only for open apps
    for app in applications::all_apps() {
        // Skip if app is not open
        if !open_app_ids.contains(&app.id) {
            continue;
        }

        for tool in app.tools {
            let tool_id = tool.id.clone();
            let tool_name = tool.name.clone();
            let app_id = app.id.clone();
            let exec_store = store.clone();

            tools.insert(
                tool_id.clone(),
                Tool::function(json!({"type": "object", "properties": {}, "required": []}))
                    .with_description(format!("{} - {}", tool_name, tool.description))
                    .with_execute(Arc::new(move |_input, _opts| {
                        // Double-check app is still open at execution time
                        let result = match exec_store.load() {
                            Ok(current_state) => {
                                let still_open = current_state
                                    .open_apps
                                    .iter()
                                    .any(|a| a.id == app_id);

                                if !still_open {
                                    Err(json!({
                                        "error": "app_closed",
                                        "appId": app_id,
                                        "hint": "The app was closed. Use window.open to reopen it."
                                    }))
                                } else {
                                    match applications::execute_tool(&tool_id, json!({})) {
                                        Some(result) => Ok(json!({
                                            "status": result.status,
                                            "message": result.message
                                        })),
                                        None => Err(json!({
                                            "error": "unknown_tool",
                                            "toolId": tool_id
                                        })),
                                    }
                                }
                            }
                            Err(error) => Err(json!({ "error": error })),
                        };
                        ToolExecutionOutput::Single(Box::pin(async move { result }))
                    })),
            );
        }
    }

    tools
}

/// Build the full tool set with all tools (for backward compatibility).
///
/// Includes window.* tools and all app tools (with runtime open-check).
/// Prefer using build_control_plane_tool_set and build_execution_plane_tool_set
/// for the new two-phase architecture.
pub fn build_tool_set(store: Arc<dyn WorkspaceStore>, workspace: WorkspaceService) -> ToolSet {
    // Start with window tools
    let mut tools = build_window_tools(store.clone(), workspace);

    // Add all app tools with runtime open-check
    for app in applications::all_apps() {
        for tool in app.tools {
            let tool_id = tool.id.clone();
            let tool_name = tool.name.clone();
            let app_id = app.id.clone();
            let tool_desc = format!(
                "{} (Requires {} to be open.)",
                tool.description, app_id
            );
            let exec_store = store.clone();

            tools.insert(
                tool_id.clone(),
                Tool::function(json!({"type": "object", "properties": {}, "required": []}))
                    .with_description(format!("{} - {}", tool_name, tool_desc))
                    .with_execute(Arc::new(move |_input, _opts| {
                        let result = match exec_store.load() {
                            Ok(workspace_state) => {
                                let app_open = workspace_state
                                    .open_apps
                                    .iter()
                                    .any(|a| a.id == app_id);

                                if !app_open {
                                    Err(json!({
                                        "error": "app_not_open",
                                        "appId": app_id,
                                        "hint": "Use window.open to open the app first."
                                    }))
                                } else {
                                    match applications::execute_tool(&tool_id, json!({})) {
                                        Some(result) => Ok(json!({
                                            "status": result.status,
                                            "message": result.message
                                        })),
                                        None => Err(json!({
                                            "error": "unknown_tool",
                                            "toolId": tool_id
                                        })),
                                    }
                                }
                            }
                            Err(error) => Err(json!({ "error": error })),
                        };
                        ToolExecutionOutput::Single(Box::pin(async move { result }))
                    })),
            );
        }
    }

    tools
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory::MemoryStore;
    use crate::workspace::types::WorkspaceState;

    #[test]
    fn test_control_plane_has_only_window_tools() {
        let store: Arc<dyn WorkspaceStore> = Arc::new(MemoryStore::default());
        let workspace = WorkspaceService::new();
        let tools = build_control_plane_tool_set(store, workspace);

        // Should have window tools
        assert!(tools.get("window_list_apps").is_some());
        assert!(tools.get("window_open").is_some());

        // Should NOT have app tools
        assert!(tools.get("spotify_play").is_none());
    }

    #[test]
    fn test_execution_plane_includes_app_tools_for_open_apps() {
        let store: Arc<dyn WorkspaceStore> = Arc::new(MemoryStore::default());
        let workspace = WorkspaceService::new();

        // Create workspace with spotify open
        let mut state = WorkspaceState::default();
        workspace.open_app(&mut state, "spotify");

        let tools = build_execution_plane_tool_set(store, workspace, &state);

        // Should have window tools
        assert!(tools.get("window_list_apps").is_some());

        // Should have spotify tools (assuming spotify app is registered)
        // This test depends on spotify being in applications::all_apps()
    }

    #[test]
    fn test_execution_plane_excludes_closed_app_tools() {
        let store: Arc<dyn WorkspaceStore> = Arc::new(MemoryStore::default());
        let workspace = WorkspaceService::new();

        // Empty workspace - no apps open
        let state = WorkspaceState::default();

        let tools = build_execution_plane_tool_set(store, workspace, &state);

        // Should have window tools
        assert!(tools.get("window_list_apps").is_some());

        // Should NOT have any app tools
        assert!(tools.get("spotify_play").is_none());
    }
}
