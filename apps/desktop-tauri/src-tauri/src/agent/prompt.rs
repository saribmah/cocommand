//! Prompt construction for the cocommand agent.
//!
//! This module orchestrates prompt building by delegating to specialized
//! submodules for base instructions, control-plane prompts, and execution-plane prompts.

mod base;
mod control;
mod execution;

// Re-export submodule functions for direct access if needed
#[allow(unused_imports)]
pub use base::{build_base_instructions, build_instructions, build_safety_rules};
#[allow(unused_imports)]
pub use control::{build_control_context, build_control_instructions, build_control_system_prompt};
#[allow(unused_imports)]
pub use execution::{
    build_execution_context, build_execution_instructions, build_execution_system_prompt,
};

use crate::workspace::types::WorkspaceSnapshot;

use super::session::SessionPhase;

/// Build the system prompt for the agent based on the current phase, context, and optional custom instructions.
///
/// This is the main entry point for prompt construction. It delegates to
/// the appropriate submodule based on the session phase.
///
/// # Arguments
/// * `phase` - The current session phase (Control or Execution)
/// * `snapshot` - The workspace snapshot for context
/// * `custom_instructions` - Optional custom instructions from AgentConfig to merge
pub fn build_system_prompt(phase: &SessionPhase, snapshot: &WorkspaceSnapshot) -> String {
    build_system_prompt_with_instructions(phase, snapshot, None)
}

/// Build the system prompt with optional custom instructions merged in.
pub fn build_system_prompt_with_instructions(
    phase: &SessionPhase,
    snapshot: &WorkspaceSnapshot,
    custom_instructions: Option<&str>,
) -> String {
    let base_instructions = base::build_base_instructions();
    let safety_rules = base::build_safety_rules();

    let base_prompt = match phase {
        SessionPhase::Control => {
            control::build_control_system_prompt(&base_instructions, &safety_rules, snapshot)
        }
        SessionPhase::Execution => {
            execution::build_execution_system_prompt(&base_instructions, &safety_rules, snapshot)
        }
    };

    // Merge custom instructions if provided (non-empty)
    if let Some(instructions) = custom_instructions {
        if !instructions.trim().is_empty() {
            return format!(
                "{}\n\n## Custom Instructions\n\n{}",
                base_prompt, instructions
            );
        }
    }

    base_prompt
}

/// Build the user prompt from the command text.
pub fn build_user_prompt(command: &str) -> String {
    command.to_string()
}

/// Build the full prompt combining system context and user command.
/// (Used for compatibility with existing agent runner)
pub fn build_prompt(command: &str, snapshot: &WorkspaceSnapshot) -> String {
    let snapshot_json = serde_json::to_string(snapshot).unwrap_or_else(|_| "{}".to_string());
    format!(
        "Workspace snapshot: {}\nUser command: {}",
        snapshot_json, command
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::types::OpenAppSummary;

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
        assert!(prompt.contains("cocommand"));
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
    fn test_build_system_prompt_with_custom_instructions() {
        let snapshot = WorkspaceSnapshot {
            focused_app: None,
            open_apps: vec![],
            staleness: "fresh".to_string(),
        };

        let prompt = build_system_prompt_with_instructions(
            &SessionPhase::Control,
            &snapshot,
            Some("Always be helpful and friendly."),
        );
        assert!(prompt.contains("Custom Instructions"));
        assert!(prompt.contains("Always be helpful and friendly."));
    }

    #[test]
    fn test_build_system_prompt_with_empty_custom_instructions() {
        let snapshot = WorkspaceSnapshot {
            focused_app: None,
            open_apps: vec![],
            staleness: "fresh".to_string(),
        };

        let prompt_without = build_system_prompt(&SessionPhase::Control, &snapshot);
        let prompt_with_empty =
            build_system_prompt_with_instructions(&SessionPhase::Control, &snapshot, Some("  "));

        // Empty instructions should not add the section
        assert!(!prompt_with_empty.contains("Custom Instructions"));
        assert_eq!(prompt_without, prompt_with_empty);
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
