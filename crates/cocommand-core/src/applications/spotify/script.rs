//! AppleScript execution utilities for Spotify.
//!
//! This module provides the shared script execution logic used by all
//! Spotify tools.

use std::process::Command;

use crate::applications::types::ToolResult;

/// Execute an AppleScript command to control Spotify.
///
/// # Arguments
/// * `action` - The Spotify action to perform (e.g., "play", "pause")
/// * `success_message` - Message to return on success
///
/// # Returns
/// A `ToolResult` indicating success or failure with appropriate message.
pub fn run_spotify_script(action: &str, success_message: &str) -> ToolResult {
    let script = format!("tell application \"Spotify\" to {}", action);
    let output = Command::new("osascript").arg("-e").arg(script).output();

    match output {
        Ok(result) if result.status.success() => ToolResult::ok(success_message),
        Ok(result) => ToolResult::error(String::from_utf8_lossy(&result.stderr).to_string()),
        Err(error) => ToolResult::error(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    // Note: These tests require Spotify to be installed and may not work in CI.
    // They are primarily for local development testing.

    #[test]
    fn test_script_format() {
        let action = "play";
        let script = format!("tell application \"Spotify\" to {}", action);
        assert!(script.contains("tell application"));
        assert!(script.contains("Spotify"));
        assert!(script.contains("play"));
    }
}
