pub use cocommand_llm::{
    LlmError, LlmKitProvider, LlmProvider, LlmSettings, LlmStream, LlmStreamEvent,
    LlmStreamOptions, LlmTool, LlmToolSet,
};

use crate::error::CoreError;
use crate::workspace::WorkspaceLLMPreferences;

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL: &str = "gpt-4o-mini";
const DEFAULT_SYSTEM_PROMPT: &str = "You are Cocommand, a helpful command assistant.";
const DEFAULT_TEMPERATURE: f64 = 0.7;
const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 80000;
const DEFAULT_MAX_STEPS: usize = 8;

pub fn settings_from_workspace(ai: &WorkspaceLLMPreferences) -> LlmSettings {
    let base_url = if ai.base_url.trim().is_empty() {
        DEFAULT_BASE_URL.to_string()
    } else {
        ai.base_url.clone()
    };
    let model = if ai.model.trim().is_empty() {
        DEFAULT_MODEL.to_string()
    } else {
        ai.model.clone()
    };
    let system_prompt = if ai.system_prompt.trim().is_empty() {
        DEFAULT_SYSTEM_PROMPT.to_string()
    } else {
        ai.system_prompt.clone()
    };
    let temperature = if ai.temperature <= 0.0 {
        DEFAULT_TEMPERATURE
    } else {
        ai.temperature
    };
    let max_output_tokens = if ai.max_output_tokens == 0 {
        DEFAULT_MAX_OUTPUT_TOKENS
    } else {
        ai.max_output_tokens
    };
    let max_steps = if ai.max_steps == 0 {
        DEFAULT_MAX_STEPS
    } else {
        ai.max_steps
    };

    LlmSettings {
        base_url,
        api_key: ai.api_key.clone().filter(|value| !value.trim().is_empty()),
        model,
        system_prompt,
        temperature,
        max_output_tokens,
        max_steps,
    }
}

impl From<LlmError> for CoreError {
    fn from(err: LlmError) -> Self {
        match err {
            LlmError::MissingApiKey => CoreError::InvalidInput("missing LLM API key".to_string()),
            LlmError::InvalidInput(msg) => CoreError::InvalidInput(msg),
            LlmError::Internal(msg) => CoreError::Internal(msg),
        }
    }
}
