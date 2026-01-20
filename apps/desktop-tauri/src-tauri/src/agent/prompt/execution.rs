//! Execution-plane prompt construction.
//!
//! The execution plane is the second phase of the agent loop where
//! app-specific tools are available in addition to window tools.
//! This phase is entered after apps have been opened in the control phase.

use crate::applications;
use crate::workspace::types::WorkspaceSnapshot;

/// Build execution-phase specific instructions.
pub fn build_execution_instructions() -> String {
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

/// Build the context section for the execution phase.
/// This includes more detailed app state since tools are now available.
pub fn build_execution_context(snapshot: &WorkspaceSnapshot) -> String {
    let snapshot_json =
        serde_json::to_string_pretty(snapshot).unwrap_or_else(|_| "{}".to_string());

    let available_apps = applications::all_apps();

    // List open apps with their available tools
    let open_app_ids: std::collections::HashSet<_> = snapshot
        .open_apps
        .iter()
        .map(|app| app.id.as_str())
        .collect();

    let mut app_sections = Vec::new();

    for app in &available_apps {
        let tools_list: Vec<String> = app
            .tools
            .iter()
            .map(|tool| format!("  - {}: {}", tool.id, tool.description))
            .collect();

        if open_app_ids.contains(app.id.as_str()) {
            app_sections.push(format!(
                "- {} (OPEN): {}\n  Available tools:\n{}",
                app.id,
                app.description,
                tools_list.join("\n")
            ));
        } else {
            app_sections.push(format!(
                "- {} (closed): {}",
                app.id, app.description
            ));
        }
    }

    format!(
        "## Current Workspace State\n\n```json\n{}\n```\n\n## Applications\n\n{}",
        snapshot_json,
        app_sections.join("\n\n")
    )
}

/// Build the full execution-phase system prompt.
pub fn build_execution_system_prompt(
    base_instructions: &str,
    safety_rules: &str,
    snapshot: &WorkspaceSnapshot,
) -> String {
    let phase_instructions = build_execution_instructions();
    let context = build_execution_context(snapshot);

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
    fn test_execution_instructions() {
        let instructions = build_execution_instructions();
        assert!(instructions.contains("Execution"));
        assert!(instructions.contains("window.*"));
        assert!(instructions.contains("open applications"));
    }

    #[test]
    fn test_execution_context_shows_open_apps() {
        let snapshot = WorkspaceSnapshot {
            focused_app: Some("spotify".to_string()),
            open_apps: vec![OpenAppSummary {
                id: "spotify".to_string(),
                summary: "Playing music".to_string(),
            }],
            staleness: "fresh".to_string(),
        };

        let context = build_execution_context(&snapshot);
        assert!(context.contains("OPEN"));
        assert!(context.contains("Workspace State"));
    }

    #[test]
    fn test_build_execution_system_prompt() {
        let snapshot = WorkspaceSnapshot {
            focused_app: Some("spotify".to_string()),
            open_apps: vec![OpenAppSummary {
                id: "spotify".to_string(),
                summary: "Open".to_string(),
            }],
            staleness: "fresh".to_string(),
        };

        let prompt = build_execution_system_prompt(
            "Base instructions",
            "Safety rules",
            &snapshot,
        );

        assert!(prompt.contains("Base instructions"));
        assert!(prompt.contains("Execution"));
        assert!(prompt.contains("Safety rules"));
    }
}
