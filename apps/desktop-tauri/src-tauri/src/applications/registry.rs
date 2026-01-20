//! Application registry for managing and executing application tools.
//!
//! This module provides the central registry for all applications and their tools.
//! It handles:
//! - Listing all available applications
//! - Looking up applications by ID
//! - Executing tools by ID

use serde_json::Value;

use super::spotify;
use super::types::{Application, ApplicationDefinition, ToolDefinition, ToolResult};

/// Get all registered applications.
///
/// Returns a list of all applications with their tools.
/// Applications are instantiated fresh each call.
pub fn all_apps() -> Vec<ApplicationDefinition> {
    let apps: Vec<Box<dyn Application>> = vec![Box::new(spotify::SpotifyApp::default())];

    apps.into_iter()
        .map(|app| ApplicationDefinition {
            id: app.id().to_string(),
            name: app.name().to_string(),
            description: app.description().to_string(),
            tools: app.tools(),
        })
        .collect()
}

/// Get all tools from all applications.
///
/// Returns a flat list of all tool definitions.
pub fn all_tools() -> Vec<ToolDefinition> {
    all_apps()
        .into_iter()
        .flat_map(|app| app.tools)
        .collect()
}

/// Find an application by its ID.
///
/// Returns the application definition if found.
pub fn app_by_id(app_id: &str) -> Option<ApplicationDefinition> {
    all_apps().into_iter().find(|app| app.id == app_id)
}

/// Execute a tool by its ID.
///
/// Routes to the appropriate tool implementation based on the tool ID.
/// Returns None if the tool is not found.
pub fn execute_tool(tool_id: &str, inputs: Value) -> Option<ToolResult> {
    use super::types::Tool;

    match tool_id {
        "spotify_play" => Some(spotify::SpotifyPlay.execute(inputs)),
        "spotify_pause" => Some(spotify::SpotifyPause.execute(inputs)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_apps_returns_spotify() {
        let apps = all_apps();
        assert!(!apps.is_empty());
        assert!(apps.iter().any(|a| a.id == "spotify"));
    }

    #[test]
    fn test_app_by_id_found() {
        let app = app_by_id("spotify");
        assert!(app.is_some());
        assert_eq!(app.unwrap().id, "spotify");
    }

    #[test]
    fn test_app_by_id_not_found() {
        let app = app_by_id("nonexistent");
        assert!(app.is_none());
    }

    #[test]
    fn test_all_tools() {
        let tools = all_tools();
        assert!(tools.iter().any(|t| t.id == "spotify_play"));
        assert!(tools.iter().any(|t| t.id == "spotify_pause"));
    }

    #[test]
    fn test_execute_tool_unknown() {
        let result = execute_tool("unknown_tool", serde_json::json!({}));
        assert!(result.is_none());
    }
}
