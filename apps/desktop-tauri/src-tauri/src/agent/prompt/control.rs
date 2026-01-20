//! Control-plane prompt construction.
//!
//! The control plane is the first phase of the agent loop where only
//! window management tools are available. The agent can list apps,
//! open/close/focus apps, and get workspace snapshots.

use crate::applications;
use crate::workspace::types::WorkspaceSnapshot;

/// Build control-phase specific instructions.
pub fn build_control_instructions() -> String {
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

/// Build the context section for the control phase.
pub fn build_control_context(snapshot: &WorkspaceSnapshot) -> String {
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

/// Build the full control-phase system prompt.
pub fn build_control_system_prompt(
    base_instructions: &str,
    safety_rules: &str,
    snapshot: &WorkspaceSnapshot,
) -> String {
    let phase_instructions = build_control_instructions();
    let context = build_control_context(snapshot);

    format!(
        "{}\n\n{}\n\n{}\n\n{}",
        base_instructions, phase_instructions, context, safety_rules
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::types::OpenAppSummary;

    #[test]
    fn test_control_instructions_lists_tools() {
        let instructions = build_control_instructions();
        assert!(instructions.contains("Control"));
        assert!(instructions.contains("window.open"));
        assert!(instructions.contains("window.close"));
        assert!(instructions.contains("window.focus"));
        assert!(instructions.contains("window.list_apps"));
        assert!(instructions.contains("window.get_snapshot"));
    }

    #[test]
    fn test_control_context_includes_snapshot() {
        let snapshot = WorkspaceSnapshot {
            focused_app: Some("spotify".to_string()),
            open_apps: vec![OpenAppSummary {
                id: "spotify".to_string(),
                summary: "Open".to_string(),
            }],
            staleness: "fresh".to_string(),
        };

        let context = build_control_context(&snapshot);
        assert!(context.contains("spotify"));
        assert!(context.contains("Workspace State"));
        assert!(context.contains("Available Applications"));
    }

    #[test]
    fn test_build_control_system_prompt() {
        let snapshot = WorkspaceSnapshot {
            focused_app: None,
            open_apps: vec![],
            staleness: "fresh".to_string(),
        };

        let prompt = build_control_system_prompt(
            "Base instructions",
            "Safety rules",
            &snapshot,
        );

        assert!(prompt.contains("Base instructions"));
        assert!(prompt.contains("Control"));
        assert!(prompt.contains("Safety rules"));
    }
}
