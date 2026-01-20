//! Base instructions and safety rules shared across all phases.
//!
//! This module provides the foundational prompt components that are
//! consistent regardless of whether the agent is in control or execution phase.

/// Build the base instructions that identify the agent and its core behavior.
pub fn build_base_instructions() -> String {
    [
        "You are cocommand, a desktop command bar agent.",
        "You help users control applications on their computer through natural language commands.",
        "You have access to window tools for managing the workspace and application tools for specific apps.",
        "Prefer the minimum tool calls needed to complete the task.",
        "If no tool applies, respond with a brief explanation.",
    ]
    .join("\n")
}

/// Build safety rules that apply to all phases.
pub fn build_safety_rules() -> String {
    [
        "## Safety Rules",
        "",
        "- Never auto-execute destructive actions without user confirmation",
        "- Prefer already-open apps over opening new ones when possible",
        "- Do not assume cached state is valid if the workspace is stale",
        "- Workspace mutations only happen through window.* tools",
        "- If the workspace is archived, require explicit restore before acting",
    ]
    .join("\n")
}

/// Build instructions string for the agent config (compatibility layer).
/// This provides a simple instruction string for AgentConfig.
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

    #[test]
    fn test_base_instructions_contains_identity() {
        let instructions = build_base_instructions();
        assert!(instructions.contains("cocommand"));
        assert!(instructions.contains("desktop command bar"));
    }

    #[test]
    fn test_safety_rules_contains_key_rules() {
        let rules = build_safety_rules();
        assert!(rules.contains("destructive actions"));
        assert!(rules.contains("window.* tools"));
        assert!(rules.contains("archived"));
    }

    #[test]
    fn test_build_instructions_compat() {
        let instructions = build_instructions();
        assert!(instructions.contains("cocommand"));
        assert!(instructions.contains("window.open"));
    }
}
