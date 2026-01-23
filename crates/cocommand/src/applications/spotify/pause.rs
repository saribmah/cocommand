//! Spotify pause tool.
//!
//! This module provides the tool for pausing playback in Spotify.

use serde_json::Value;

use crate::applications::types::{Tool, ToolResult};

use super::script::run_spotify_script;

/// Tool ID for the pause tool.
pub const TOOL_ID: &str = "spotify_pause";

/// Tool for pausing Spotify playback.
pub struct SpotifyPause;

impl Tool for SpotifyPause {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Pause"
    }

    fn description(&self) -> &str {
        "Pause playback in Spotify."
    }

    fn execute(&self, _inputs: Value) -> ToolResult {
        run_spotify_script("pause", "Spotify pause triggered.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = SpotifyPause;
        assert_eq!(tool.id(), "spotify_pause");
    }

    #[test]
    fn test_tool_name() {
        let tool = SpotifyPause;
        assert_eq!(tool.name(), "Pause");
    }

    #[test]
    fn test_tool_description() {
        let tool = SpotifyPause;
        assert!(tool.description().contains("Pause"));
    }
}
