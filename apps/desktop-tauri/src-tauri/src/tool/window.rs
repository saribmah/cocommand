//! Window management tools module.
//!
//! This module aggregates all window.* tools that form the control plane.
//! These tools are always available regardless of which apps are open.

mod close;
mod focus;
mod list_apps;
mod open;
mod restore;
mod snapshot;

use std::sync::Arc;

use llm_kit_core::ToolSet;

use crate::storage::WorkspaceStore;
use crate::workspace::service::WorkspaceService;

// Re-export tool IDs for reference
pub use close::TOOL_ID as CLOSE_TOOL_ID;
pub use focus::TOOL_ID as FOCUS_TOOL_ID;
pub use list_apps::TOOL_ID as LIST_APPS_TOOL_ID;
pub use open::TOOL_ID as OPEN_TOOL_ID;
pub use restore::TOOL_ID as RESTORE_TOOL_ID;
pub use snapshot::TOOL_ID as SNAPSHOT_TOOL_ID;

/// Build the complete set of window management tools.
///
/// These tools form the control plane and are always available:
/// - window.list_apps: List all available applications
/// - window.get_snapshot: Get the current workspace state
/// - window.open: Open an application (mounts its tools)
/// - window.close: Close an application (unmounts its tools)
/// - window.focus: Focus an already-open application
/// - window.restore_workspace: Restore a soft-reset or archived workspace
pub fn build_window_tools(store: Arc<dyn WorkspaceStore>, workspace: WorkspaceService) -> ToolSet {
    let mut tools = ToolSet::new();

    // Add list_apps (no store/workspace needed)
    let (id, tool) = list_apps::build();
    tools.insert(id, tool);

    // Add snapshot
    let (id, tool) = snapshot::build(store.clone(), workspace.clone());
    tools.insert(id, tool);

    // Add open
    let (id, tool) = open::build(store.clone(), workspace.clone());
    tools.insert(id, tool);

    // Add close
    let (id, tool) = close::build(store.clone(), workspace.clone());
    tools.insert(id, tool);

    // Add focus
    let (id, tool) = focus::build(store.clone(), workspace.clone());
    tools.insert(id, tool);

    // Add restore_workspace
    let (id, tool) = restore::build(store.clone(), workspace.clone());
    tools.insert(id, tool);

    tools
}

/// Get a list of all window tool IDs.
///
/// Useful for checking if a tool belongs to the window module.
pub fn all_tool_ids() -> Vec<&'static str> {
    vec![
        LIST_APPS_TOOL_ID,
        SNAPSHOT_TOOL_ID,
        OPEN_TOOL_ID,
        CLOSE_TOOL_ID,
        FOCUS_TOOL_ID,
        RESTORE_TOOL_ID,
    ]
}

/// Check if a tool ID is a window tool.
pub fn is_window_tool(tool_id: &str) -> bool {
    tool_id.starts_with("window_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory::MemoryStore;

    #[test]
    fn test_build_window_tools_has_all_tools() {
        let store: Arc<dyn WorkspaceStore> = Arc::new(MemoryStore::default());
        let workspace = WorkspaceService::new();
        let tools = build_window_tools(store, workspace);

        assert!(tools.get("window_list_apps").is_some());
        assert!(tools.get("window_get_snapshot").is_some());
        assert!(tools.get("window_open").is_some());
        assert!(tools.get("window_close").is_some());
        assert!(tools.get("window_focus").is_some());
        assert!(tools.get("window_restore_workspace").is_some());
    }

    #[test]
    fn test_all_tool_ids() {
        let ids = all_tool_ids();
        assert_eq!(ids.len(), 6);
        assert!(ids.contains(&"window_list_apps"));
        assert!(ids.contains(&"window_get_snapshot"));
        assert!(ids.contains(&"window_open"));
        assert!(ids.contains(&"window_close"));
        assert!(ids.contains(&"window_focus"));
        assert!(ids.contains(&"window_restore_workspace"));
    }

    #[test]
    fn test_is_window_tool() {
        assert!(is_window_tool("window_open"));
        assert!(is_window_tool("window_close"));
        assert!(!is_window_tool("spotify_play"));
        assert!(!is_window_tool("calendar_create"));
    }
}
