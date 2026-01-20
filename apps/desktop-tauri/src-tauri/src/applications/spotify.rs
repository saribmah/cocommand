//! Spotify application integration.
//!
//! Provides tools for controlling Spotify playback via AppleScript.

use serde_json::Value;
use std::process::Command;

use super::types::{tool_definition, Application, Tool, ToolDefinition, ToolResult};

/// Spotify application.
#[derive(Default)]
pub struct SpotifyApp;

/// Tool for playing/resuming Spotify playback.
pub struct SpotifyPlay;

/// Tool for pausing Spotify playback.
pub struct SpotifyPause;

impl Tool for SpotifyPlay {
    fn id(&self) -> &str {
        "spotify_play"
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

impl Tool for SpotifyPause {
    fn id(&self) -> &str {
        "spotify_pause"
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

/// Execute an AppleScript command to control Spotify.
fn run_spotify_script(action: &str, success_message: &str) -> ToolResult {
    let script = format!("tell application \"Spotify\" to {}", action);
    let output = Command::new("osascript").arg("-e").arg(script).output();

    match output {
        Ok(result) if result.status.success() => ToolResult::ok(success_message),
        Ok(result) => ToolResult::error(String::from_utf8_lossy(&result.stderr).to_string()),
        Err(error) => ToolResult::error(error.to_string()),
    }
}

impl Application for SpotifyApp {
    fn id(&self) -> &str {
        "spotify"
    }

    fn name(&self) -> &str {
        "Spotify"
    }

    fn description(&self) -> &str {
        "Control playback and library in Spotify."
    }

    fn tools(&self) -> Vec<ToolDefinition> {
        vec![tool_definition(&SpotifyPlay), tool_definition(&SpotifyPause)]
    }
}
