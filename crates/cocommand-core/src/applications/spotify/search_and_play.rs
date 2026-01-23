//! Spotify search and play tool.
//!
//! This module provides the tool for searching Spotify and playing the results.
//! It supports searching for tracks or playlists by query.
//! Uses Spotify Web API to resolve actual URIs, then AppleScript to play them.

use serde_json::{json, Value};
use std::process::Command;

use crate::applications::types::{Tool, ToolResult};

use super::api::{is_api_available, search_playlist, search_track};

/// Tool ID for the search_and_play tool.
pub const TOOL_ID: &str = "spotify_search_and_play";

/// Tool for searching and playing content on Spotify.
pub struct SpotifySearchAndPlay;

impl Tool for SpotifySearchAndPlay {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Search and Play"
    }

    fn description(&self) -> &str {
        "Search Spotify for music and play the results. Use this for queries like 'focus music', 'piano music', 'jazz', 'workout playlist', etc. Searches Spotify and plays the best matching content."
    }

    fn schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query (e.g., 'focus music', 'piano', 'jazz', 'workout playlist')"
                },
                "content_type": {
                    "type": "string",
                    "enum": ["track", "playlist"],
                    "description": "Type of content to search for. Use 'playlist' for mood/genre queries like 'focus music', 'workout'. Use 'track' for specific songs. Defaults to 'playlist'."
                }
            },
            "required": ["query"]
        }))
    }

    fn execute(&self, inputs: Value) -> ToolResult {
        let query = inputs
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if query.is_empty() {
            return ToolResult::error("Search query is required");
        }

        let content_type = inputs
            .get("content_type")
            .and_then(|v| v.as_str())
            .unwrap_or("playlist");

        // Try to use Spotify Web API if available
        if is_api_available() {
            // Search using the Spotify Web API to get the actual URI
            let search_result = match content_type {
                "track" => search_track(query),
                _ => search_playlist(query).or_else(|| search_track(query)),
            };

            if let Some(result) = search_result {
                // Determine if this is a track (use play track) or playlist (use open location + play)
                let is_track = result.uri.starts_with("spotify:track:");

                let script = if is_track {
                    format!(
                        r#"
                        tell application "Spotify"
                            activate
                            delay 0.3
                            play track "{}"
                        end tell
                        "#,
                        result.uri
                    )
                } else {
                    format!(
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
                    )
                };

                let output = Command::new("osascript").arg("-e").arg(&script).output();

                return match output {
                    Ok(cmd_result) if cmd_result.status.success() => {
                        let msg = match result.artist {
                            Some(artist) => format!("Now playing: {} by {}", result.name, artist),
                            None => format!("Now playing: {}", result.name),
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

        // Fallback: open search in Spotify UI and auto-play
        // This happens when API is unavailable or search returned no results
        let encoded_query = urlencoding_simple(query);
        let search_uri = format!("spotify:search:{}", encoded_query);

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
                ToolResult::ok(format!(
                    "Searching for '{}' on Spotify and attempting to play. If nothing plays, please select a result manually.",
                    query
                ))
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
        let tool = SpotifySearchAndPlay;
        assert_eq!(tool.id(), "spotify_search_and_play");
    }

    #[test]
    fn test_tool_name() {
        let tool = SpotifySearchAndPlay;
        assert_eq!(tool.name(), "Search and Play");
    }

    #[test]
    fn test_tool_description() {
        let tool = SpotifySearchAndPlay;
        assert!(tool.description().contains("Search"));
        assert!(tool.description().contains("play"));
    }

    #[test]
    fn test_tool_has_schema() {
        let tool = SpotifySearchAndPlay;
        let schema = tool.schema();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&json!("query")));
    }

    #[test]
    fn test_execute_empty_query() {
        let tool = SpotifySearchAndPlay;
        let result = tool.execute(json!({}));
        assert_eq!(result.status, "error");
        assert!(result.message.contains("required"));
    }

    #[test]
    fn test_urlencoding_simple() {
        assert_eq!(urlencoding_simple("focus music"), "focus%20music");
        assert_eq!(urlencoding_simple("rock & roll"), "rock%20%26%20roll");
    }
}
