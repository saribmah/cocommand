use std::sync::Arc;

use llm_kit_openai_compatible::OpenAICompatibleClient;
use llm_kit_provider::LanguageModel;

use crate::error::{CoreError, CoreResult};
use crate::workspace::WorkspaceLLMPreferences;

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL: &str = "gpt-4o-mini";
const DEFAULT_SYSTEM_PROMPT: &str = "You are Cocommand, a helpful command assistant.";
const DEFAULT_TEMPERATURE: f64 = 0.7;
const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 80000;
const DEFAULT_MAX_STEPS: usize = 8;

#[derive(Debug, Clone)]
pub struct LlmSettings {
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub system_prompt: String,
    pub temperature: f64,
    pub max_output_tokens: u32,
    pub max_steps: usize,
}

impl LlmSettings {
    pub fn from_workspace(ai: &WorkspaceLLMPreferences) -> Self {
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

        Self {
            base_url,
            api_key: ai.api_key.clone().filter(|value| !value.trim().is_empty()),
            model,
            system_prompt,
            temperature,
            max_output_tokens,
            max_steps,
        }
    }
}

pub fn build_model(settings: &LlmSettings) -> CoreResult<Arc<dyn LanguageModel>> {
    let api_key = settings
        .api_key
        .clone()
        .ok_or_else(|| CoreError::InvalidInput("missing LLM API key".to_string()))?;
    let provider = OpenAICompatibleClient::new()
        .base_url(&settings.base_url)
        .api_key(&api_key)
        .build();
    Ok(provider.chat_model(&settings.model))
}
