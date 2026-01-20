use crate::applications;
use crate::workspace::types::WorkspaceSnapshot;

use super::session::SessionPhase;

/// Build the system prompt for the agent based on the current phase and context
pub fn build_system_prompt(phase: &SessionPhase, snapshot: &WorkspaceSnapshot) -> String {
    let base_instructions = build_base_instructions();
    let phase_instructions = build_phase_instructions(phase);
    let context = build_context(snapshot);
    let safety_rules = build_safety_rules();

    format!(
        "{}\n\n{}\n\n{}\n\n{}",
        base_instructions, phase_instructions, context, safety_rules
    )
}

/// Build the user prompt from the command text
pub fn build_user_prompt(command: &str) -> String {
    command.to_string()
}

/// Build the full prompt combining system context and user command
/// (Used for compatibility with existing agent runner)
pub fn build_prompt(command: &str, snapshot: &WorkspaceSnapshot) -> String {
    let snapshot_json = serde_json::to_string(snapshot).unwrap_or_else(|_| "{}".to_string());
    format!(
        "Workspace snapshot: {}\nUser command: {}",
        snapshot_json, command
    )
}

fn build_base_instructions() -> String {
    [
        "You are cocommand, a desktop command bar agent.",
        "You help users control applications on their computer through natural language commands.",
        "You have access to window tools for managing the workspace and application tools for specific apps.",
        "Prefer the minimum tool calls needed to complete the task.",
        "If no tool applies, respond with a brief explanation.",
    ]
    .join("\n")
}

fn build_phase_instructions(phase: &SessionPhase) -> String {
    match phase {
        SessionPhase::Control => {
            [
                "## Current Phase: Control",
                "",
                "You are in the control phase. You only have access to window management tools:",
                "- window.list_apps: List all available applications",
                "- window.get_snapshot: Get the current workspace state",
                "- window.open: Open an application (this mounts its tools)",
                "- window.close: Close an application (this unmounts its tools)",
                "- window.focus: Focus on an already-open application",
                "",
                "To use an application's tools, you must first open it with window.open.",
                "After opening an app, you will gain access to its specific tools.",
            ]
            .join("\n")
        }
        SessionPhase::Execution => {
            [
                "## Current Phase: Execution",
                "",
                "You are in the execution phase. You have access to:",
                "- All window management tools (window.*)",
                "- Tools for all currently open applications",
                "",
                "You can now execute application-specific actions.",
                "If you need tools from a closed app, use window.open to open it first.",
            ]
            .join("\n")
        }
    }
}

fn build_context(snapshot: &WorkspaceSnapshot) -> String {
    let snapshot_json =
        serde_json::to_string_pretty(snapshot).unwrap_or_else(|_| "{}".to_string());

    let available_apps = applications::all_apps();
    let app_list: Vec<String> = available_apps
        .iter()
        .map(|app| format!("- {}: {}", app.id, app.description))
        .collect();

    format!(
        "## Current Workspace State\n\n```json\n{}\n```\n\n## Available Applications\n\n{}",
        snapshot_json,
        app_list.join("\n")
    )
}

fn build_safety_rules() -> String {
    [
        "## Safety Rules",
        "",
        "- Never auto-execute destructive actions without user confirmation",
        "- Prefer already-open apps over opening new ones when possible",
        "- Do not assume cached state is valid if the workspace is stale",
        "- Workspace mutations only happen through window.* tools",
    ]
    .join("\n")
}

/// Build instructions string for the agent config (compatibility layer)
pub fn build_instructions() -> String {
    [
        "You are cocommand, a desktop command bar agent.",
        "You have access to window tools and application tools.",
        "Use window.open to open an app before calling its tools.",
        "Prefer the minimum tool calls needed to complete the task.",
        "If no tool applies, respond with a brief explanation.",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::types::{OpenAppSummary, WorkspaceSnapshot};

    #[test]
    fn test_build_system_prompt_control_phase() {
        let snapshot = WorkspaceSnapshot {
            focused_app: None,
            open_apps: vec![],
            staleness: "fresh".to_string(),
        };

        let prompt = build_system_prompt(&SessionPhase::Control, &snapshot);
        assert!(prompt.contains("Control"));
        assert!(prompt.contains("window.open"));
    }

    #[test]
    fn test_build_system_prompt_execution_phase() {
        let snapshot = WorkspaceSnapshot {
            focused_app: Some("spotify".to_string()),
            open_apps: vec![OpenAppSummary {
                id: "spotify".to_string(),
                summary: "Open".to_string(),
            }],
            staleness: "fresh".to_string(),
        };

        let prompt = build_system_prompt(&SessionPhase::Execution, &snapshot);
        assert!(prompt.contains("Execution"));
        assert!(prompt.contains("spotify"));
    }

    #[test]
    fn test_build_prompt_compat() {
        let snapshot = WorkspaceSnapshot {
            focused_app: Some("spotify".to_string()),
            open_apps: vec![],
            staleness: "fresh".to_string(),
        };

        let prompt = build_prompt("play music", &snapshot);
        assert!(prompt.contains("play music"));
        assert!(prompt.contains("focusedApp"));
    }
}
