use std::sync::Arc;

use llm_kit_core::agent::{Agent, AgentCallParameters, AgentInterface, AgentSettings};
use llm_kit_core::ToolSet;

use crate::llm::client::LlmClient;
use crate::storage::WorkspaceStore;
use crate::tool_registry::registry::{build_control_plane_tool_set, build_execution_plane_tool_set};
use crate::workspace::service::WorkspaceService;

use super::config::AgentConfig;
use super::prompt;
use super::session::Session;

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
        let workspace_state = self.store.load()?;
        let snapshot = self.workspace_service.snapshot(&workspace_state);

        // Phase 1: Control plane
        // Only window.* tools are available
        session.set_phase(SessionPhase::Control);
        let control_tools = build_control_plane_tool_set(
            self.store.clone(),
            self.workspace_service.clone(),
        );

        let control_prompt = prompt::build_prompt(command, &snapshot);
        let control_result = self.run_agent(&control_tools, &control_prompt).await?;
        session.increment_turn();

        // Check if any apps were opened during control phase
        let updated_workspace = self.store.load()?;
        let has_open_apps = !updated_workspace.open_apps.is_empty();

        // If apps were opened, transition to execution phase
        if has_open_apps && session.can_continue() {
            session.set_phase(SessionPhase::Execution);
            let updated_snapshot = self.workspace_service.snapshot(&updated_workspace);

            // Build execution plane tools (window.* + app tools for open apps)
            let execution_tools = build_execution_plane_tool_set(
                self.store.clone(),
                self.workspace_service.clone(),
                &updated_workspace,
            );

            // Build new prompt with updated snapshot
            let execution_prompt = prompt::build_prompt(command, &updated_snapshot);
            let execution_result = self.run_agent(&execution_tools, &execution_prompt).await?;
            session.increment_turn();

            return Ok(ProcessResult {
                success: true,
                output: execution_result,
                phase_used: SessionPhase::Execution,
                turns_used: session.turn_count,
            });
        }

        // No apps opened, return control phase result
        Ok(ProcessResult {
            success: true,
            output: control_result,
            phase_used: SessionPhase::Control,
            turns_used: session.turn_count,
        })
    }

    /// Process with explicit phase control (for advanced use cases)
    pub async fn process_in_phase(
        &self,
        command: &str,
        phase: SessionPhase,
    ) -> Result<ProcessResult, String> {
        let workspace_state = self.store.load()?;
        let snapshot = self.workspace_service.snapshot(&workspace_state);

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

        let prompt = prompt::build_prompt(command, &snapshot);
        let result = self.run_agent(&tools, &prompt).await?;

        Ok(ProcessResult {
            success: true,
            output: result,
            phase_used: phase,
            turns_used: 1,
        })
    }

    /// Run the agent with a given tool set and prompt
    async fn run_agent(&self, tools: &ToolSet, prompt: &str) -> Result<String, String> {
        let runner = Agent::new(
            AgentSettings::new(self.llm.model())
                .with_id(self.agent_config.id.clone())
                .with_instructions(self.agent_config.instructions.clone())
                .with_tools(tools.clone())
                .with_temperature(self.agent_config.temperature)
                .with_max_output_tokens(self.agent_config.max_output_tokens),
        );

        let output = runner
            .generate(AgentCallParameters::from_text(prompt))
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
