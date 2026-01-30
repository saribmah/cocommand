use std::env;
use std::sync::Arc;

use llm_kit_openai_compatible::OpenAICompatibleClient;
use llm_kit_provider::LanguageModel;

use crate::error::{CoreError, CoreResult};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL: &str = "gpt-4o-mini";
const DEFAULT_SYSTEM_PROMPT: &str = "You are Cocommand, a helpful command assistant.";
const DEFAULT_TEMPERATURE: f64 = 0.7;
const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 80000;
const DEFAULT_MAX_STEPS: usize = 8;

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
    pub fn from_env() -> CoreResult<Self> {
        let base_url = env::var("COCOMMAND_LLM_BASE_URL")
            .or_else(|_| env::var("OPENAI_BASE_URL"))
            .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        let api_key = env::var("COCOMMAND_LLM_API_KEY")
            .or_else(|_| env::var("OPENAI_API_KEY"))
            .ok();
        let model = env::var("COCOMMAND_LLM_MODEL")
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let system_prompt = env::var("COCOMMAND_LLM_SYSTEM_PROMPT")
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.to_string());
        let temperature = env::var("COCOMMAND_LLM_TEMPERATURE")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(DEFAULT_TEMPERATURE);
        let max_output_tokens = env::var("COCOMMAND_LLM_MAX_OUTPUT_TOKENS")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(DEFAULT_MAX_OUTPUT_TOKENS);
        let max_steps = env::var("COCOMMAND_LLM_MAX_STEPS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(DEFAULT_MAX_STEPS);

        Ok(Self {
            base_url,
            api_key,
            model,
            system_prompt,
            temperature,
            max_output_tokens,
            max_steps,
        })
    }
}

pub fn build_model(settings: &LlmSettings) -> CoreResult<Arc<dyn LanguageModel>> {
    let api_key = settings.api_key.clone().ok_or_else(|| {
        CoreError::InvalidInput("missing LLM API key".to_string())
    })?;
    let provider = OpenAICompatibleClient::new()
        .base_url(&settings.base_url)
        .api_key(&api_key)
        .build();
    Ok(provider.chat_model(&settings.model))
}
