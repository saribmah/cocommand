//! Application registry for managing and executing application tools.
//!
//! This module provides the central registry for all applications and their tools.
//! It handles:
//! - Listing all available applications
//! - Looking up applications by ID
//! - Executing tools by ID
//!
//! # Architecture
//!
//! The registry follows opencode's pattern of:
//! - Apps as capability packs that provide tool bundles
//! - Fresh instantiation per call (no caching)
//! - Pattern-matched dispatch for tool execution

use serde_json::Value;

use super::spotify::{self, PAUSE_TOOL_ID, PLAY_TOOL_ID, PLAY_TRACK_TOOL_ID};
use super::types::{Application, ApplicationDefinition, Tool, ToolDefinition, ToolResult};

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

/// Get tools for a specific application by ID.
///
/// Returns the tool definitions for the application if found.
pub fn tools_for_app(app_id: &str) -> Option<Vec<ToolDefinition>> {
    app_by_id(app_id).map(|app| app.tools)
}

/// Check if a tool belongs to an application.
///
/// Returns true if the tool ID starts with the app ID prefix.
pub fn tool_belongs_to_app(tool_id: &str, app_id: &str) -> bool {
    tool_id.starts_with(&format!("{}_", app_id))
}

/// Execute a tool by its ID.
///
/// Routes to the appropriate tool implementation based on the tool ID.
/// Returns None if the tool is not found.
pub fn execute_tool(tool_id: &str, inputs: Value) -> Option<ToolResult> {
    match tool_id {
        id if id == PLAY_TOOL_ID => Some(spotify::SpotifyPlay.execute(inputs)),
        id if id == PAUSE_TOOL_ID => Some(spotify::SpotifyPause.execute(inputs)),
        id if id == PLAY_TRACK_TOOL_ID => Some(spotify::SpotifyPlayTrack.execute(inputs)),
        _ => None,
    }
}

/// Get the app ID from a tool ID.
///
/// Extracts the app prefix from tool IDs that follow the `appid_action` pattern.
pub fn app_id_from_tool(tool_id: &str) -> Option<&str> {
    tool_id.split('_').next()
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

    #[test]
    fn test_tools_for_app() {
        let tools = tools_for_app("spotify");
        assert!(tools.is_some());
        let tools = tools.unwrap();
        assert_eq!(tools.len(), 3);
    }

    #[test]
    fn test_play_track_tool_has_schema() {
        let tools = all_tools();
        let play_track = tools.iter().find(|t| t.id == "spotify_play_track").unwrap();
        assert!(play_track.schema.is_some());
    }

    #[test]
    fn test_tool_belongs_to_app() {
        assert!(tool_belongs_to_app("spotify_play", "spotify"));
        assert!(tool_belongs_to_app("spotify_pause", "spotify"));
        assert!(!tool_belongs_to_app("spotify_play", "apple_music"));
    }

    #[test]
    fn test_app_id_from_tool() {
        assert_eq!(app_id_from_tool("spotify_play"), Some("spotify"));
        assert_eq!(app_id_from_tool("unknown"), Some("unknown"));
    }
}
