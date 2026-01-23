//! Spotify play album tool.
//!
//! This module provides the tool for playing a specific album on Spotify.
//! Uses Spotify Web API to resolve the album URI, then AppleScript to play.

use serde_json::{json, Value};
use std::process::Command;

use crate::applications::types::{Tool, ToolResult};

use super::api::{is_api_available, search_album};

/// Tool ID for the play_album tool.
pub const TOOL_ID: &str = "spotify_play_album";

/// Tool for playing a specific album on Spotify.
pub struct SpotifyPlayAlbum;

impl Tool for SpotifyPlayAlbum {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Play Album"
    }

    fn description(&self) -> &str {
        "Play a specific album on Spotify. Use this when the user wants to listen to a particular album (e.g., 'play the album Abbey Road', 'play 1989 album')."
    }

    fn schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "album": {
                    "type": "string",
                    "description": "Name of the album to play (e.g., 'Abbey Road', '1989', 'Dark Side of the Moon')"
                },
                "artist": {
                    "type": "string",
                    "description": "Optional: Artist name to help find the correct album"
                }
            },
            "required": ["album"]
        }))
    }

    fn execute(&self, inputs: Value) -> ToolResult {
        let album = inputs
            .get("album")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if album.is_empty() {
            return ToolResult::error("Album name is required");
        }

        let artist = inputs
            .get("artist")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        // Try to use Spotify Web API if available
        if is_api_available() {
            if let Some(result) = search_album(album, artist) {
                // Use AppleScript with open location + play for album URIs
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
                        let msg = match result.artist {
                            Some(artist_name) => {
                                format!("Now playing album: {} by {}", result.name, artist_name)
                            }
                            None => format!("Now playing album: {}", result.name),
                        };
                        ToolResult::ok(msg)
                    }
                    Ok(cmd_result) => {
                        ToolResult::error(String::from_utf8_lossy(&cmd_result.stderr).to_string())
                    }
                    Err(error) => ToolResult::error(error.to_string()),
                };
            }
        }

        // Fallback: open album search in Spotify UI and auto-play
        let encoded_album = urlencoding_simple(album);
        let search_uri = match artist {
            Some(a) => {
                let encoded_artist = urlencoding_simple(a);
                format!(
                    "spotify:search:album%3A{}%20artist%3A{}",
                    encoded_album, encoded_artist
                )
            }
            None => format!("spotify:search:album%3A{}", encoded_album),
        };

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
            Ok(cmd_result) if cmd_result.status.success() => {
                let msg = match artist {
                    Some(a) => format!(
                        "Opened Spotify search for album '{}' by '{}' and started playback. If nothing plays, please select the album manually.",
                        album, a
                    ),
                    None => format!(
                        "Opened Spotify search for album '{}' and started playback. If nothing plays, please select the album manually.",
                        album
                    ),
                };
                ToolResult::ok(msg)
            }
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
        let tool = SpotifyPlayAlbum;
        assert_eq!(tool.id(), "spotify_play_album");
    }

    #[test]
    fn test_tool_name() {
        let tool = SpotifyPlayAlbum;
        assert_eq!(tool.name(), "Play Album");
    }

    #[test]
    fn test_tool_description() {
        let tool = SpotifyPlayAlbum;
        assert!(tool.description().contains("album"));
    }

    #[test]
    fn test_tool_has_schema() {
        let tool = SpotifyPlayAlbum;
        let schema = tool.schema();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["album"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&json!("album")));
    }

    #[test]
    fn test_execute_empty_album() {
        let tool = SpotifyPlayAlbum;
        let result = tool.execute(json!({}));
        assert_eq!(result.status, "error");
        assert!(result.message.contains("required"));
    }

    #[test]
    fn test_urlencoding_simple() {
        assert_eq!(urlencoding_simple("Abbey Road"), "Abbey%20Road");
    }
}
