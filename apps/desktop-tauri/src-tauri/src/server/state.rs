use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct ToolDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Clone)]
pub struct AppState {
    pub tools: Vec<ToolDefinition>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            tools: vec![
                ToolDefinition {
                    id: "spotify.play".to_string(),
                    name: "Play".to_string(),
                    description: "Resume playback in Spotify".to_string(),
                },
                ToolDefinition {
                    id: "finder.move".to_string(),
                    name: "Move File".to_string(),
                    description: "Move files in Finder".to_string(),
                },
            ],
        }
    }
}
