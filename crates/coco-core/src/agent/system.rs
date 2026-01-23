//! System prompt assembly for the cocommand agent.
//!
//! This module provides a clean interface for building system prompts,
//! separating the "what to include" from "how to format it". It mirrors
//! opencode's system vs session separation pattern.
//!
//! # Architecture
//!
//! The system prompt is assembled from:
//! - Identity: Who the agent is
//! - Capabilities: What tools/phases are available
//! - Context: Current workspace state
//! - Safety: Rules and guardrails
//! - Custom: User/app-provided instructions
//!
//! # Usage
//!
//! ```ignore
//! let system = SystemPromptBuilder::new()
//!     .with_phase(&SessionPhase::Control)
//!     .with_snapshot(&snapshot)
//!     .with_custom_instructions(Some("Be concise"))
//!     .build();
//! ```

use crate::workspace::types::WorkspaceSnapshot;

use super::prompt;
use super::session::SessionPhase;

/// Builder for system prompts with explicit configuration.
///
/// Provides a structured way to assemble system prompts with all required
/// context while maintaining separation between prompt content and assembly logic.
#[derive(Default)]
pub struct SystemPromptBuilder<'a> {
    phase: Option<&'a SessionPhase>,
    snapshot: Option<&'a WorkspaceSnapshot>,
    custom_instructions: Option<&'a str>,
}

impl<'a> SystemPromptBuilder<'a> {
    /// Create a new builder with default (empty) configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the session phase for the prompt.
    pub fn with_phase(mut self, phase: &'a SessionPhase) -> Self {
        self.phase = Some(phase);
        self
    }

    /// Set the workspace snapshot for context.
    pub fn with_snapshot(mut self, snapshot: &'a WorkspaceSnapshot) -> Self {
        self.snapshot = Some(snapshot);
        self
    }

    /// Set optional custom instructions to append.
    pub fn with_custom_instructions(mut self, instructions: Option<&'a str>) -> Self {
        self.custom_instructions = instructions;
        self
    }

    /// Build the final system prompt string.
    ///
    /// Panics if phase or snapshot are not set.
    pub fn build(self) -> String {
        let phase = self.phase.expect("phase is required for system prompt");
        let snapshot = self.snapshot.expect("snapshot is required for system prompt");

        prompt::build_system_prompt_with_instructions(phase, snapshot, self.custom_instructions)
    }

    /// Try to build the system prompt, returning None if required fields are missing.
    pub fn try_build(self) -> Option<String> {
        let phase = self.phase?;
        let snapshot = self.snapshot?;

        Some(prompt::build_system_prompt_with_instructions(
            phase,
            snapshot,
            self.custom_instructions,
        ))
    }
}

/// Assemble a complete system prompt for the given phase and context.
///
/// This is a convenience function that wraps the builder pattern
/// for simple use cases.
pub fn assemble_system_prompt(
    phase: &SessionPhase,
    snapshot: &WorkspaceSnapshot,
    custom_instructions: Option<&str>,
) -> String {
    SystemPromptBuilder::new()
        .with_phase(phase)
        .with_snapshot(snapshot)
        .with_custom_instructions(custom_instructions)
        .build()
}

/// Get identity components for the system prompt.
pub mod identity {
    /// The agent's name.
    pub const NAME: &str = "cocommand";

    /// The agent's role description.
    pub const ROLE: &str = "desktop command bar agent";

    /// Build a brief identity string.
    pub fn brief() -> String {
        format!("You are {}, a {}.", NAME, ROLE)
    }
}

/// Get capability descriptions based on phase.
pub mod capabilities {
    use super::SessionPhase;

    /// Describe available capabilities for the current phase.
    pub fn describe(phase: &SessionPhase) -> String {
        match phase {
            SessionPhase::Control => {
                "You have access to window.* tools for managing the workspace. \
                 Use window.open to open applications before using their tools."
                    .to_string()
            }
            SessionPhase::Execution => {
                "You have access to window.* tools plus application-specific tools \
                 for any open applications. Execute tasks using the appropriate tools."
                    .to_string()
            }
        }
    }

    /// List the tool categories available in the phase.
    pub fn tool_categories(phase: &SessionPhase) -> Vec<&'static str> {
        match phase {
            SessionPhase::Control => vec!["window.*"],
            SessionPhase::Execution => vec!["window.*", "app-specific tools"],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::types::OpenAppSummary;

    fn test_snapshot() -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            focused_app: None,
            open_apps: vec![],
            staleness: "fresh".to_string(),
        }
    }

    fn test_snapshot_with_app() -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            focused_app: Some("spotify".to_string()),
            open_apps: vec![OpenAppSummary {
                id: "spotify".to_string(),
                summary: "Open".to_string(),
            }],
            staleness: "fresh".to_string(),
        }
    }

    #[test]
    fn test_builder_control_phase() {
        let snapshot = test_snapshot();
        let prompt = SystemPromptBuilder::new()
            .with_phase(&SessionPhase::Control)
            .with_snapshot(&snapshot)
            .build();

        assert!(prompt.contains("Control"));
        assert!(prompt.contains("window"));
    }

    #[test]
    fn test_builder_execution_phase() {
        let snapshot = test_snapshot_with_app();
        let prompt = SystemPromptBuilder::new()
            .with_phase(&SessionPhase::Execution)
            .with_snapshot(&snapshot)
            .build();

        assert!(prompt.contains("Execution"));
        assert!(prompt.contains("spotify"));
    }

    #[test]
    fn test_builder_with_custom_instructions() {
        let snapshot = test_snapshot();
        let prompt = SystemPromptBuilder::new()
            .with_phase(&SessionPhase::Control)
            .with_snapshot(&snapshot)
            .with_custom_instructions(Some("Be very concise"))
            .build();

        assert!(prompt.contains("Custom Instructions"));
        assert!(prompt.contains("Be very concise"));
    }

    #[test]
    fn test_try_build_missing_phase() {
        let snapshot = test_snapshot();
        let result = SystemPromptBuilder::new()
            .with_snapshot(&snapshot)
            .try_build();

        assert!(result.is_none());
    }

    #[test]
    fn test_assemble_convenience() {
        let snapshot = test_snapshot();
        let prompt = assemble_system_prompt(&SessionPhase::Control, &snapshot, None);

        assert!(prompt.contains("cocommand"));
    }

    #[test]
    fn test_identity_brief() {
        let brief = identity::brief();
        assert!(brief.contains("cocommand"));
        assert!(brief.contains("desktop command bar agent"));
    }

    #[test]
    fn test_capabilities_describe() {
        let control = capabilities::describe(&SessionPhase::Control);
        assert!(control.contains("window"));

        let execution = capabilities::describe(&SessionPhase::Execution);
        assert!(execution.contains("application-specific"));
    }

    #[test]
    fn test_capabilities_tool_categories() {
        let control_cats = capabilities::tool_categories(&SessionPhase::Control);
        assert_eq!(control_cats.len(), 1);
        assert_eq!(control_cats[0], "window.*");

        let exec_cats = capabilities::tool_categories(&SessionPhase::Execution);
        assert_eq!(exec_cats.len(), 2);
    }
}
