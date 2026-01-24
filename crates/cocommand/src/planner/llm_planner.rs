use std::sync::Arc;

use async_trait::async_trait;
use llm_kit_core::agent::{Agent, AgentCallParameters, AgentSettings};
use llm_kit_core::generate_text::{step_count_is, StepResult, StopCondition};
use llm_kit_core::output::Output;
use llm_kit_provider::LanguageModel;
use llm_kit_core::AgentInterface;
use llm_kit_provider::error::ProviderError;
use serde_json::Value;
use std::error::Error;

use super::plan::{Plan, PlannedToolCall};
use super::types::{PlanMetadata, PlannerError, PlannerInput, PlannerOutput};

/// LLM-backed planner that uses llm-kit to execute tool calls.
pub struct LlmPlanner {
    model: Arc<dyn LanguageModel>,
    instructions: Option<String>,
    max_steps: u32,
}

impl LlmPlanner {
    pub fn new(model: Arc<dyn LanguageModel>) -> Self {
        Self {
            model,
            instructions: None,
            max_steps: 6,
        }
    }

    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    pub fn with_max_steps(mut self, max_steps: u32) -> Self {
        self.max_steps = max_steps;
        self
    }
}

#[async_trait]
impl super::Planner for LlmPlanner {
    async fn plan(&self, input: PlannerInput) -> Result<PlannerOutput, PlannerError> {
        let Some(toolset) = input.toolset else {
            return Ok(PlannerOutput::new(Plan::empty(), PlanMetadata::stub(), None, vec![]));
        };

        println!("[planner] llm planner invoked");

        let stop_conditions: Vec<Arc<dyn StopCondition>> = vec![
            Arc::new(step_count_is(self.max_steps as usize)),
            Arc::new(ApprovalRequiredStop),
        ];

        let mut settings = AgentSettings::new(Arc::clone(&self.model))
            .with_tools(toolset)
            .with_stop_when(stop_conditions);

        if let Some(instructions) = &self.instructions {
            settings = settings.with_instructions(instructions.clone());
        }

        let agent = Agent::new(settings);

        let prompt = input.command.raw_text;
        let result = agent
            .generate(AgentCallParameters::from_text(prompt))
            .map_err(|e| {
                log_llm_error("generate", &e);
                PlannerError::Internal(e.to_string())
            })?
            .execute()
            .await
            .map_err(|e| {
                log_llm_error("execute", &e);
                PlannerError::ProviderUnavailable(e.to_string())
            })?;

        let mut steps = Vec::new();
        let mut tool_errors = Vec::new();
        for step in &result.steps {
            for tool_call in step.tool_calls() {
                steps.push(PlannedToolCall {
                    tool_id: tool_call.tool_name.clone(),
                    args: tool_call.input.clone(),
                });
            }
            for part in &step.content {
                if let Output::ToolError(error) = part {
                    tool_errors.push(error.error.clone());
                }
            }
        }

        let plan = if steps.is_empty() {
            Plan::empty()
        } else {
            Plan::new(steps)
        };

        let total_usage = result.total_usage;
        let metadata = PlanMetadata {
            planner_id: "llm".to_string(),
            model: result.response.model_id,
            reasoning: result.reasoning_text,
            prompt_tokens: u64_to_u32(total_usage.input_tokens),
            completion_tokens: u64_to_u32(total_usage.output_tokens),
            total_tokens: u64_to_u32(total_usage.total()),
        };

        println!(
            "[planner] llm response_text_len={} tool_errors={}",
            result.text.len(),
            tool_errors.len()
        );
        Ok(PlannerOutput::new(
            plan,
            metadata,
            Some(result.text),
            tool_errors,
        ))
    }
}

fn u64_to_u32(value: u64) -> Option<u32> {
    Some(u32::try_from(value).unwrap_or(u32::MAX))
}

struct ApprovalRequiredStop;

#[async_trait]
impl StopCondition for ApprovalRequiredStop {
    async fn check(&self, steps: &[StepResult]) -> bool {
        steps
            .iter()
            .any(|step| step_has_approval_required_error(step))
    }
}

fn step_has_approval_required_error(step: &StepResult) -> bool {
    step.content.iter().any(|part| match part {
        Output::ToolError(error) => is_approval_required(&error.error),
        _ => false,
    })
}

fn is_approval_required(error: &Value) -> bool {
    error
        .as_object()
        .and_then(|obj| obj.get("type"))
        .and_then(|value| value.as_str())
        == Some("approval_required")
}

fn log_llm_error(stage: &str, err: &(dyn Error + 'static)) {
    println!("[planner] llm {stage} error={err:?}");
    let mut current: Option<&(dyn Error + 'static)> = Some(err);
    let mut depth = 0;
    while let Some(e) = current {
        if depth > 0 {
            println!("[planner] llm error cause={e:?}");
        }
        if let Some(provider_error) = e.downcast_ref::<ProviderError>() {
            println!(
                "[planner] provider_error status={:?} url={:?}",
                provider_error.status_code(),
                provider_error.url()
            );
            if let Some(body) = provider_error.response_body() {
                println!("[planner] provider_error_body={body}");
            }
        }
        current = e.source();
        depth += 1;
    }
}
