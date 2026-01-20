//! Tool registry for building tool sets.
//!
//! This module provides builders for different tool set configurations:
//! - Control plane: Only window.* tools (for workspace management)
//! - Execution plane: Window.* plus app-specific tools
//!
//! # Architecture
//!
//! The registry follows opencode's pattern of:
//! - Phase-based tool assembly (control vs execution)
//! - Runtime validation (app open checks at execution time)
//! - Helper functions for common operations

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

// ============================================================================
// Core Tool Set Builders
// ============================================================================

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

    // Add tools for open apps only
    let open_app_ids = collect_open_app_ids(workspace_state);
    add_app_tools(&mut tools, store, &open_app_ids);

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
    add_all_app_tools_with_runtime_check(&mut tools, store);

    tools
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Collect the set of open app IDs from the workspace state.
fn collect_open_app_ids(workspace_state: &WorkspaceState) -> HashSet<String> {
    workspace_state
        .open_apps
        .iter()
        .map(|app| app.id.clone())
        .collect()
}

/// Add tools for specified open apps to the tool set.
fn add_app_tools(
    tools: &mut ToolSet,
    store: Arc<dyn WorkspaceStore>,
    open_app_ids: &HashSet<String>,
) {
    for app in applications::all_apps() {
        // Skip if app is not open
        if !open_app_ids.contains(&app.id) {
            continue;
        }

        for tool in app.tools {
            let tool_entry = build_app_tool_with_open_check(
                tool.id.clone(),
                tool.name.clone(),
                tool.description.clone(),
                app.id.clone(),
                store.clone(),
            );
            tools.insert(tool.id.clone(), tool_entry);
        }
    }
}

/// Add all app tools with runtime open-check (legacy mode).
fn add_all_app_tools_with_runtime_check(tools: &mut ToolSet, store: Arc<dyn WorkspaceStore>) {
    for app in applications::all_apps() {
        for tool in app.tools {
            let tool_desc = format!(
                "{} (Requires {} to be open.)",
                tool.description, app.id
            );
            let tool_entry = build_app_tool_with_open_check(
                tool.id.clone(),
                tool.name.clone(),
                tool_desc,
                app.id.clone(),
                store.clone(),
            );
            tools.insert(tool.id.clone(), tool_entry);
        }
    }
}

/// Build a tool entry with runtime app-open validation.
fn build_app_tool_with_open_check(
    tool_id: String,
    tool_name: String,
    tool_description: String,
    app_id: String,
    store: Arc<dyn WorkspaceStore>,
) -> Tool {
    Tool::function(json!({"type": "object", "properties": {}, "required": []}))
        .with_description(format!("{} - {}", tool_name, tool_description))
        .with_execute(Arc::new(move |_input, _opts| {
            let result = execute_tool_with_app_check(&store, &tool_id, &app_id);
            ToolExecutionOutput::Single(Box::pin(async move { result }))
        }))
}

/// Execute a tool with app-open validation.
fn execute_tool_with_app_check(
    store: &Arc<dyn WorkspaceStore>,
    tool_id: &str,
    app_id: &str,
) -> Result<serde_json::Value, serde_json::Value> {
    match store.load() {
        Ok(current_state) => {
            let app_open = current_state.open_apps.iter().any(|a| a.id == app_id);

            if !app_open {
                Err(json!({
                    "error": "app_closed",
                    "appId": app_id,
                    "hint": "The app was closed. Use window.open to reopen it."
                }))
            } else {
                match applications::execute_tool(tool_id, json!({})) {
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
    }
}

// ============================================================================
// Query Functions
// ============================================================================

/// Check if a tool set contains a specific tool.
pub fn tool_set_contains(tools: &ToolSet, tool_id: &str) -> bool {
    tools.get(tool_id).is_some()
}

/// Get the list of tool IDs in a tool set.
pub fn tool_set_ids(tools: &ToolSet) -> Vec<String> {
    tools.keys().cloned().collect()
}

// ============================================================================
// Tests
// ============================================================================

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
        assert!(tool_set_contains(&tools, "window_list_apps"));
        assert!(tool_set_contains(&tools, "window_open"));

        // Should NOT have app tools
        assert!(!tool_set_contains(&tools, "spotify_play"));
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
        assert!(tool_set_contains(&tools, "window_list_apps"));

        // Should have spotify tools
        assert!(tool_set_contains(&tools, "spotify_play"));
        assert!(tool_set_contains(&tools, "spotify_pause"));
    }

    #[test]
    fn test_execution_plane_excludes_closed_app_tools() {
        let store: Arc<dyn WorkspaceStore> = Arc::new(MemoryStore::default());
        let workspace = WorkspaceService::new();

        // Empty workspace - no apps open
        let state = WorkspaceState::default();

        let tools = build_execution_plane_tool_set(store, workspace, &state);

        // Should have window tools
        assert!(tool_set_contains(&tools, "window_list_apps"));

        // Should NOT have any app tools
        assert!(!tool_set_contains(&tools, "spotify_play"));
    }

    #[test]
    fn test_collect_open_app_ids() {
        let workspace = WorkspaceService::new();
        let mut state = WorkspaceState::default();
        workspace.open_app(&mut state, "spotify");

        let ids = collect_open_app_ids(&state);

        assert!(ids.contains("spotify"));
        assert!(!ids.contains("unknown"));
    }

    #[test]
    fn test_tool_set_ids() {
        let store: Arc<dyn WorkspaceStore> = Arc::new(MemoryStore::default());
        let workspace = WorkspaceService::new();
        let tools = build_control_plane_tool_set(store, workspace);

        let ids = tool_set_ids(&tools);

        assert!(ids.contains(&"window_list_apps".to_string()));
        assert!(ids.contains(&"window_open".to_string()));
    }
}
