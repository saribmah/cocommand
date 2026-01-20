use super::config::LlmConfig;
use llm_kit_core::{prompt::Prompt, GenerateText};
use llm_kit_openai_compatible::OpenAICompatibleClient;
use llm_kit_provider::language_model::LanguageModel;
use std::sync::Arc;

#[derive(Clone)]
pub struct LlmClient {
    config: LlmConfig,
    model: Arc<dyn LanguageModel>,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        // let mut builder = OpenAICompatibleClient::new();
        // if !config.api_key.is_empty() {
        //     // builder = builder.api_key(config.api_key.clone());
        //     builder = builder.api_key("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");
        // }
        // builder = builder.api_key("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");
        // if let Some(base_url) = config.base_url.as_ref() {
        //     // builder = builder.base_url(base_url);
        //     builder = builder.base_url("https://openrouter.ai/api/v1")
        // }
        // builder = builder.base_url("https://openrouter.ai/api/v1");
        let provider = OpenAICompatibleClient::new()
            .base_url("https://openrouter.ai/api/v1")
            .api_key("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")
            .build();
        // let provider = builder.build();
        let model = provider.chat_model(config.model.clone());
        Self { config, model }
    }

    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    pub fn model(&self) -> Arc<dyn LanguageModel> {
        self.model.clone()
    }

    pub async fn generate_text(&self, prompt: &str) -> Result<String, String> {
        let result = GenerateText::new(self.model.clone(), Prompt::text(prompt))
            .temperature(0.2)
            .max_output_tokens(200)
            .execute()
            .await
            .map_err(|error| error.to_string())?;
        Ok(result.text)
    }
}
