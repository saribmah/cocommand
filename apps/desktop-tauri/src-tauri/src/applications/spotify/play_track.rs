//! Spotify play_track tool.
//!
//! This module provides the tool for playing a specific track in Spotify.

use serde_json::{json, Value};

use crate::applications::types::{Tool, ToolResult};

use super::script::run_spotify_script;

/// Tool ID for the play_track tool.
pub const TOOL_ID: &str = "spotify_play_track";

/// Tool for playing a specific track in Spotify.
pub struct SpotifyPlayTrack;

impl Tool for SpotifyPlayTrack {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Play Track"
    }

    fn description(&self) -> &str {
        "Play a specific track in Spotify by its URI."
    }

    fn schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "uri": {
                    "type": "string",
                    "description": "Spotify track URI (e.g., 'spotify:track:4iV5W9uYEdYUVa79Axb7Rh')"
                }
            },
            "required": ["uri"]
        }))
    }

    fn execute(&self, inputs: Value) -> ToolResult {
        let uri = inputs
            .get("uri")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if uri.is_empty() {
            return ToolResult::error("Track URI is required");
        }

        if !uri.starts_with("spotify:track:") {
            return ToolResult::error("Invalid Spotify track URI format");
        }

        let action = format!("play track \"{}\"", uri);
        run_spotify_script(&action, &format!("Now playing track: {}", uri))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = SpotifyPlayTrack;
        assert_eq!(tool.id(), "spotify_play_track");
    }

    #[test]
    fn test_tool_name() {
        let tool = SpotifyPlayTrack;
        assert_eq!(tool.name(), "Play Track");
    }

    #[test]
    fn test_tool_description() {
        let tool = SpotifyPlayTrack;
        assert!(tool.description().contains("specific track"));
    }

    #[test]
    fn test_tool_has_schema() {
        let tool = SpotifyPlayTrack;
        let schema = tool.schema();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["uri"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&json!("uri")));
    }

    #[test]
    fn test_execute_empty_uri() {
        let tool = SpotifyPlayTrack;
        let result = tool.execute(json!({}));
        assert_eq!(result.status, "error");
        assert!(result.message.contains("required"));
    }

    #[test]
    fn test_execute_invalid_uri() {
        let tool = SpotifyPlayTrack;
        let result = tool.execute(json!({"uri": "invalid"}));
        assert_eq!(result.status, "error");
        assert!(result.message.contains("Invalid"));
    }
}
