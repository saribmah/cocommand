//! Spotify play tool.
//!
//! This module provides the tool for resuming playback in Spotify.

use serde_json::Value;

use crate::applications::types::{Tool, ToolResult};

use super::script::run_spotify_script;

/// Tool ID for the play tool.
pub const TOOL_ID: &str = "spotify_play";

/// Tool for playing/resuming Spotify playback.
pub struct SpotifyPlay;

impl Tool for SpotifyPlay {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Play"
    }

    fn description(&self) -> &str {
        "Resume playback in Spotify."
    }

    fn execute(&self, _inputs: Value) -> ToolResult {
        run_spotify_script("play", "Spotify play triggered.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = SpotifyPlay;
        assert_eq!(tool.id(), "spotify_play");
    }

    #[test]
    fn test_tool_name() {
        let tool = SpotifyPlay;
        assert_eq!(tool.name(), "Play");
    }

    #[test]
    fn test_tool_description() {
        let tool = SpotifyPlay;
        assert!(tool.description().contains("Resume"));
    }
}
