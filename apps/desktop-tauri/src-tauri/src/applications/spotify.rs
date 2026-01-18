use super::{tool_definition, Application, Tool, ToolDefinition, ToolResult};
use serde_json::Value;

#[derive(Default)]
pub struct SpotifyApp;

pub struct SpotifyPlay;
pub struct SpotifyPause;

impl Tool for SpotifyPlay {
    fn id(&self) -> &str {
        "spotify.play"
    }

    fn name(&self) -> &str {
        "Play"
    }

    fn description(&self) -> &str {
        "Resume playback in Spotify."
    }

    fn execute(&self, _inputs: Value) -> ToolResult {
        ToolResult {
            status: "ok".to_string(),
            message: "Spotify play triggered (stub).".to_string(),
        }
    }
}

impl Tool for SpotifyPause {
    fn id(&self) -> &str {
        "spotify.pause"
    }

    fn name(&self) -> &str {
        "Pause"
    }

    fn description(&self) -> &str {
        "Pause playback in Spotify."
    }

    fn execute(&self, _inputs: Value) -> ToolResult {
        ToolResult {
            status: "ok".to_string(),
            message: "Spotify pause triggered (stub).".to_string(),
        }
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
        vec![
            tool_definition(&SpotifyPlay),
            tool_definition(&SpotifyPause),
        ]
    }
}
