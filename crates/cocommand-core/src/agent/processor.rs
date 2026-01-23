//! Agent processor for the control→execution loop.
//!
//! This module manages the multi-step agent loop with explicit phases:
//! - Control phase: Only window.* tools available for workspace management
//! - Execution phase: Window.* tools + app-specific tools for open apps
//!
//! # Architecture (OpenCode-Inspired)
//!
//! The processor follows a two-phase architecture with smart phase selection:
//!
//! ## Phase Selection Logic
//! 1. If workspace is archived → Control Phase with restore tool only
//! 2. If apps are already open → Direct Execution Phase (skip control)
//! 3. If no apps open → Control Phase first, then Execution if apps opened
//!
//! ## Phases
//! 1. **Control Phase**: Window.* tools only for workspace management
//!    - User can open/close apps, manage workspace
//!    - On `window.open`, the app's tools become available in next phase
//!
//! 2. **Execution Phase**: Window.* + all open app tools
//!    - Includes window.* tools + all open app tools
//!    - If `window.open` is called here, we rebuild tools and continue
//!
//! This enables efficient command handling - "pause Spotify" executes immediately
//! when Spotify is open, while "open Safari" goes through control first.

use std::sync::Arc;

use llm_kit_core::agent::{Agent, AgentCallParameters, AgentInterface, AgentSettings};
use llm_kit_core::ToolSet;

use crate::llm::client::LlmClient;
use crate::storage::WorkspaceStore;
use crate::tool::{build_control_plane_tool_set, build_execution_plane_tool_set, build_archived_tool_set};
use crate::workspace::service::WorkspaceService;

use super::config::AgentConfig;
use super::context::ContextBuilder;
use super::session::Session;
use super::system::assemble_system_prompt;

// Re-export SessionPhase for use in api.rs
pub use super::session::SessionPhase;

/// Result of processing a command
#[derive(Debug)]
pub struct ProcessResult {
    pub success: bool,
    pub output: String,
    pub phase_used: SessionPhase,
    pub turns_used: u32,
}

/// Processor manages the multi-step agent loop with control→execution phases
pub struct Processor {
    llm: LlmClient,
    store: Arc<dyn WorkspaceStore>,
    workspace_service: WorkspaceService,
    agent_config: AgentConfig,
}

impl Processor {
    pub fn new(
        llm: LlmClient,
        store: Arc<dyn WorkspaceStore>,
        workspace_service: WorkspaceService,
        agent_config: AgentConfig,
    ) -> Self {
        Self {
            llm,
            store,
            workspace_service,
            agent_config,
        }
    }

    /// Process a user command through the control→execution loop
    ///
    /// Phase selection logic:
    /// 1. If workspace is archived → Control Phase with restore tool only
    /// 2. If apps are already open → Direct Execution Phase (skip control)
    /// 3. If no apps open → Control Phase first, then Execution if apps opened
    pub async fn process(&self, command: &str) -> Result<ProcessResult, String> {
        let session_id = format!(
            "session_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let mut session = Session::new(session_id).with_max_turns(10);

        // Load initial workspace state
        let mut workspace_state = self.store.load()?;

        // Build context with lifecycle checks
        let context_builder = ContextBuilder::new(&self.workspace_service);
        let context = context_builder.build(&mut workspace_state, SessionPhase::Control);

        // Save updated staleness state
        self.store.save(&workspace_state)?;

        // Build user command with lifecycle context if applicable
        let user_command = if let Some(ref msg) = context.lifecycle_message {
            format!("Note: {}\n\n{}", msg, command)
        } else {
            command.to_string()
        };

        // Case 1: Archived workspace - only allow restore tool
        if context.is_archived {
            session.set_phase(SessionPhase::Control);
            let archived_tools = build_archived_tool_set(
                self.store.clone(),
                self.workspace_service.clone(),
            );

            let archived_command = format!(
                "Note: Workspace is archived. You can ONLY use window_restore_workspace to recover. All other actions are blocked until restored.\n\n{}",
                command
            );

            let result = self
                .run_agent(&archived_tools, &SessionPhase::Control, &context.snapshot, &archived_command)
                .await?;
            session.increment_turn();

            return Ok(ProcessResult {
                success: true,
                output: result,
                phase_used: SessionPhase::Control,
                turns_used: session.turn_count,
            });
        }

        // Check if apps are already open
        let has_apps_initially = !workspace_state.open_apps.is_empty();

        // Case 2: Apps already open - go directly to Execution Phase
        if has_apps_initially {
            return self.run_execution_loop(&mut session, &workspace_state, &user_command).await;
        }

        // Case 3: No apps open - run Control Phase first
        session.set_phase(SessionPhase::Control);
        let control_tools = build_control_plane_tool_set(
            self.store.clone(),
            self.workspace_service.clone(),
        );

        let control_result = self
            .run_agent(&control_tools, &SessionPhase::Control, &context.snapshot, &user_command)
            .await?;
        session.increment_turn();

        // Reload workspace state after control phase
        let current_workspace = self.store.load()?;

        // Check if apps are now open
        let has_apps_now = !current_workspace.open_apps.is_empty();

        // If no apps open after control, return control result
        if !has_apps_now {
            return Ok(ProcessResult {
                success: true,
                output: control_result,
                phase_used: SessionPhase::Control,
                turns_used: session.turn_count,
            });
        }

        // Apps were opened during control - transition to Execution Phase
        self.run_execution_loop(&mut session, &current_workspace, &user_command).await
    }

    /// Run the execution loop with app tools
    ///
    /// This loop handles the case where new apps are opened during execution,
    /// rebuilding the tool set to include newly available tools.
    async fn run_execution_loop(
        &self,
        session: &mut Session,
        initial_workspace: &crate::workspace::types::WorkspaceState,
        user_command: &str,
    ) -> Result<ProcessResult, String> {
        session.set_phase(SessionPhase::Execution);

        let mut current_workspace = initial_workspace.clone();
        let mut execution_result = String::new();
        let mut previous_app_count = current_workspace.open_apps.len();

        // Execution loop: re-run if window.open adds new apps
        while session.can_continue() {
            let current_snapshot = self.workspace_service.snapshot(&current_workspace);

            // Build execution plane tools (window.* + app tools for open apps)
            let execution_tools = build_execution_plane_tool_set(
                self.store.clone(),
                self.workspace_service.clone(),
                &current_workspace,
            );

            // Run execution phase with the full user_command (includes lifecycle context)
            execution_result = self
                .run_agent(&execution_tools, &SessionPhase::Execution, &current_snapshot, user_command)
                .await?;
            session.increment_turn();

            // Reload workspace to check if apps changed
            current_workspace = self.store.load()?;
            let current_app_count = current_workspace.open_apps.len();

            // If app count changed, new apps were opened - loop to include their tools
            if current_app_count > previous_app_count {
                previous_app_count = current_app_count;
                // Continue loop to rebuild tools with newly opened apps
                continue;
            }

            // No new apps opened, execution is complete
            break;
        }

        Ok(ProcessResult {
            success: true,
            output: execution_result,
            phase_used: SessionPhase::Execution,
            turns_used: session.turn_count,
        })
    }

    /// Process with explicit phase control (for advanced use cases)
    ///
    /// This method respects workspace lifecycle rules while allowing
    /// explicit phase selection.
    pub async fn process_in_phase(
        &self,
        command: &str,
        phase: SessionPhase,
    ) -> Result<ProcessResult, String> {
        let mut workspace_state = self.store.load()?;

        // Build context with lifecycle checks
        let context_builder = ContextBuilder::new(&self.workspace_service);
        let context = context_builder.build(&mut workspace_state, phase.clone());

        // Save updated state
        self.store.save(&workspace_state)?;

        // Check if workspace is archived
        if context.is_archived {
            return Ok(ProcessResult {
                success: false,
                output: context.lifecycle_message.unwrap_or_else(|| {
                    "Workspace is archived.".to_string()
                }),
                phase_used: phase,
                turns_used: 0,
            });
        }

        let tools = match phase {
            SessionPhase::Control => {
                build_control_plane_tool_set(self.store.clone(), self.workspace_service.clone())
            }
            SessionPhase::Execution => build_execution_plane_tool_set(
                self.store.clone(),
                self.workspace_service.clone(),
                &workspace_state,
            ),
        };

        let user_command = if let Some(ref msg) = context.lifecycle_message {
            format!("Note: {}\n\n{}", msg, command)
        } else {
            command.to_string()
        };

        let result = self
            .run_agent(&tools, &phase, &context.snapshot, &user_command)
            .await?;

        Ok(ProcessResult {
            success: true,
            output: result,
            phase_used: phase,
            turns_used: 1,
        })
    }

    /// Run the agent with a given tool set and phase
    async fn run_agent(
        &self,
        tools: &ToolSet,
        phase: &SessionPhase,
        snapshot: &crate::workspace::types::WorkspaceSnapshot,
        command: &str,
    ) -> Result<String, String> {
        // Build the system prompt using the modular prompt system
        // Merge in custom instructions from AgentConfig if non-empty
        let custom_instructions = if self.agent_config.instructions.trim().is_empty() {
            None
        } else {
            Some(self.agent_config.instructions.as_str())
        };
        let system_prompt = assemble_system_prompt(phase, snapshot, custom_instructions);

        let runner = Agent::new(
            AgentSettings::new(self.llm.model())
                .with_id(self.agent_config.id.clone())
                .with_instructions(system_prompt)
                .with_tools(tools.clone())
                .with_temperature(self.agent_config.temperature)
                .with_max_output_tokens(self.agent_config.max_output_tokens),
        );

        let output = runner
            .generate(AgentCallParameters::from_text(command))
            .map_err(|error| error.to_string())?
            .execute()
            .await
            .map_err(|error| error.to_string())?;

        Ok(output.text)
    }
}

/// Convenience function for one-shot command processing
/// This is the main entry point for the /command endpoint
pub async fn process_command(
    llm: &LlmClient,
    store: Arc<dyn WorkspaceStore>,
    workspace_service: WorkspaceService,
    agent_config: AgentConfig,
    command: &str,
) -> Result<ProcessResult, String> {
    let processor = Processor::new(
        llm.clone(),
        store,
        workspace_service,
        agent_config,
    );

    processor.process(command).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_result_creation() {
        let result = ProcessResult {
            success: true,
            output: "Done".to_string(),
            phase_used: SessionPhase::Control,
            turns_used: 1,
        };

        assert!(result.success);
        assert_eq!(result.output, "Done");
        assert_eq!(result.phase_used, SessionPhase::Control);
    }
}
