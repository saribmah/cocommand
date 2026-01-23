use llm_kit_core::agent::{Agent, AgentCallParameters, AgentInterface, AgentSettings};
use llm_kit_core::ToolSet;

use super::config::AgentConfig;
use crate::llm::client::LlmClient;

pub async fn run_command(
    llm: &LlmClient,
    agent: &AgentConfig,
    tools: ToolSet,
    prompt: String,
) -> Result<String, String> {
    let runner = Agent::new(
        AgentSettings::new(llm.model())
            .with_id(agent.id.clone())
            .with_instructions(agent.instructions.clone())
            .with_tools(tools)
            .with_temperature(agent.temperature)
            .with_max_output_tokens(agent.max_output_tokens),
    );

    let output = runner
        .generate(AgentCallParameters::from_text(prompt))
        .map_err(|error| error.to_string())?
        .execute()
        .await
        .map_err(|error| error.to_string())?;

    Ok(output.text)
}
