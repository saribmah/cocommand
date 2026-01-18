use super::{tool_definition, Application, Tool, ToolDefinition, ToolResult};
use serde_json::Value;
use std::process::Command;

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
        run_spotify_script("play", "Spotify play triggered.")
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
        run_spotify_script("pause", "Spotify pause triggered.")
    }
}

fn run_spotify_script(action: &str, success_message: &str) -> ToolResult {
    let script = format!("tell application \"Spotify\" to {}", action);
    let output = Command::new("osascript").arg("-e").arg(script).output();
    match output {
        Ok(result) if result.status.success() => ToolResult {
            status: "ok".to_string(),
            message: success_message.to_string(),
        },
        Ok(result) => ToolResult {
            status: "error".to_string(),
            message: String::from_utf8_lossy(&result.stderr).to_string(),
        },
        Err(error) => ToolResult {
            status: "error".to_string(),
            message: error.to_string(),
        },
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
