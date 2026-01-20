use crate::workspace::types::WorkspaceSnapshot;

use super::config::AgentConfig;

pub fn default_agent() -> AgentConfig {
    AgentConfig {
        id: "primary".to_string(),
        instructions: build_instructions(),
        temperature: 0.2,
        max_output_tokens: 600,
    }
}

pub fn build_prompt(command: &str, snapshot: &WorkspaceSnapshot) -> String {
    let snapshot_json = serde_json::to_string(snapshot).unwrap_or_else(|_| "{}".to_string());
    format!(
        "Workspace snapshot: {}\nUser command: {}",
        snapshot_json, command
    )
}

fn build_instructions() -> String {
    [
        "You are cocommand, a desktop command bar agent.",
        "You have access to window tools and application tools.",
        "Use window.open to open an app before calling its tools.",
        "Prefer the minimum tool calls needed to complete the task.",
        "If no tool applies, respond with a brief explanation.",
    ]
    .join("\n")
}
