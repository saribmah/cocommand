//! Spotify play artist tool.
//!
//! This module provides the tool for playing music by a specific artist on Spotify.
//! Uses Spotify Web API to resolve the artist URI, then AppleScript to play.

use serde_json::{json, Value};
use std::process::Command;

use crate::applications::types::{Tool, ToolResult};

use super::api::{is_api_available, search_artist};

/// Tool ID for the play_artist tool.
pub const TOOL_ID: &str = "spotify_play_artist";

/// Tool for playing music by a specific artist on Spotify.
pub struct SpotifyPlayArtist;

impl Tool for SpotifyPlayArtist {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Play Artist"
    }

    fn description(&self) -> &str {
        "Play music by a specific artist on Spotify. Use this when the user wants to listen to a particular artist (e.g., 'play Taylor Swift', 'play music by The Beatles')."
    }

    fn schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "artist": {
                    "type": "string",
                    "description": "Name of the artist to play (e.g., 'Taylor Swift', 'The Beatles', 'Daft Punk')"
                }
            },
            "required": ["artist"]
        }))
    }

    fn execute(&self, inputs: Value) -> ToolResult {
        let artist = inputs
            .get("artist")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if artist.is_empty() {
            return ToolResult::error("Artist name is required");
        }

        // Try to use Spotify Web API if available
        if is_api_available() {
            if let Some(result) = search_artist(artist) {
                // Use AppleScript with open location + play for artist URIs
                let script = format!(
                    r#"
                    tell application "Spotify"
                        activate
                        delay 0.3
                        open location "{}"
                        delay 0.5
                        play
                    end tell
                    "#,
                    result.uri
                );

                let output = Command::new("osascript").arg("-e").arg(&script).output();

                return match output {
                    Ok(cmd_result) if cmd_result.status.success() => {
                        ToolResult::ok(format!("Now playing music by: {}", result.name))
                    }
                    Ok(cmd_result) => {
                        ToolResult::error(String::from_utf8_lossy(&cmd_result.stderr).to_string())
                    }
                    Err(error) => ToolResult::error(error.to_string()),
                };
            }
        }

        // Fallback: open artist search in Spotify UI and auto-play
        let encoded_artist = urlencoding_simple(artist);
        let search_uri = format!("spotify:search:artist%3A{}", encoded_artist);

        let script = format!(
            r#"
            tell application "Spotify"
                activate
                delay 0.3
                open location "{}"
                delay 1.0
                play
            end tell
            "#,
            search_uri
        );

        let output = Command::new("osascript").arg("-e").arg(&script).output();

        match output {
            Ok(cmd_result) if cmd_result.status.success() => ToolResult::ok(format!(
                "Opened Spotify search for artist '{}' and started playback. If nothing plays, please select the artist manually.",
                artist
            )),
            Ok(cmd_result) => {
                ToolResult::error(String::from_utf8_lossy(&cmd_result.stderr).to_string())
            }
            Err(error) => ToolResult::error(error.to_string()),
        }
    }
}

/// Simple URL encoding for Spotify URIs.
fn urlencoding_simple(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '"' => "%22".to_string(),
            '#' => "%23".to_string(),
            '&' => "%26".to_string(),
            '\'' => "%27".to_string(),
            '/' => "%2F".to_string(),
            ':' => "%3A".to_string(),
            '?' => "%3F".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = SpotifyPlayArtist;
        assert_eq!(tool.id(), "spotify_play_artist");
    }

    #[test]
    fn test_tool_name() {
        let tool = SpotifyPlayArtist;
        assert_eq!(tool.name(), "Play Artist");
    }

    #[test]
    fn test_tool_description() {
        let tool = SpotifyPlayArtist;
        assert!(tool.description().contains("artist"));
    }

    #[test]
    fn test_tool_has_schema() {
        let tool = SpotifyPlayArtist;
        let schema = tool.schema();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["artist"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&json!("artist")));
    }

    #[test]
    fn test_execute_empty_artist() {
        let tool = SpotifyPlayArtist;
        let result = tool.execute(json!({}));
        assert_eq!(result.status, "error");
        assert!(result.message.contains("required"));
    }

    #[test]
    fn test_urlencoding_simple() {
        assert_eq!(urlencoding_simple("Taylor Swift"), "Taylor%20Swift");
    }
}
