//! Spotify open tool.
//!
//! This module provides the tool for opening/activating the Spotify application.

use serde_json::Value;

use crate::applications::types::{Tool, ToolResult};

use super::script::run_spotify_script;

/// Tool ID for the open tool.
pub const TOOL_ID: &str = "spotify_open";

/// Tool for opening/activating the Spotify application.
pub struct SpotifyOpen;

impl Tool for SpotifyOpen {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Open"
    }

    fn description(&self) -> &str {
        "Open and activate the Spotify application. Launches Spotify if not running, or brings it to focus if already open."
    }

    fn execute(&self, _inputs: Value) -> ToolResult {
        run_spotify_script("activate", "Spotify app activated.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = SpotifyOpen;
        assert_eq!(tool.id(), "spotify_open");
    }

    #[test]
    fn test_tool_name() {
        let tool = SpotifyOpen;
        assert_eq!(tool.name(), "Open");
    }

    #[test]
    fn test_tool_description() {
        let tool = SpotifyOpen;
        assert!(tool.description().contains("Open"));
        assert!(tool.description().contains("activate"));
    }
}
