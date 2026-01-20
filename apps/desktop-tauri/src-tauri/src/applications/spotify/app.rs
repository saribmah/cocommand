//! Spotify application definition.
//!
//! This module provides the SpotifyApp struct that implements the Application trait.

use crate::applications::types::{tool_definition, Application, ToolDefinition};

use super::pause::SpotifyPause;
use super::play::SpotifyPlay;
use super::play_track::SpotifyPlayTrack;

/// Application ID for Spotify.
pub const APP_ID: &str = "spotify";

/// Spotify application.
#[derive(Default)]
pub struct SpotifyApp;

impl Application for SpotifyApp {
    fn id(&self) -> &str {
        APP_ID
    }

    fn name(&self) -> &str {
        "Spotify"
    }

    fn description(&self) -> &str {
        "Control playback and library in Spotify."
    }

    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            tool_definition(&SpotifyPlay),
            tool_definition(&SpotifyPause),
            tool_definition(&SpotifyPlayTrack),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_id() {
        let app = SpotifyApp::default();
        assert_eq!(app.id(), "spotify");
    }

    #[test]
    fn test_app_name() {
        let app = SpotifyApp::default();
        assert_eq!(app.name(), "Spotify");
    }

    #[test]
    fn test_app_tools() {
        let app = SpotifyApp::default();
        let tools = app.tools();

        assert_eq!(tools.len(), 3);
        assert!(tools.iter().any(|t| t.id == "spotify_play"));
        assert!(tools.iter().any(|t| t.id == "spotify_pause"));
        assert!(tools.iter().any(|t| t.id == "spotify_play_track"));
    }

    #[test]
    fn test_play_track_has_schema() {
        let app = SpotifyApp::default();
        let tools = app.tools();
        let play_track = tools.iter().find(|t| t.id == "spotify_play_track").unwrap();

        // Verify that play_track has a schema (unlike play/pause which have none)
        assert!(play_track.schema.is_some());
        let schema = play_track.schema.as_ref().unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["uri"].is_object());
    }
}
