use std::sync::Arc;

use llm_kit_core::{StreamText, prompt::Prompt, step_count_is, stream_text::StreamTextResult};
use llm_kit_provider::LanguageModel;

use crate::error::{CoreError, CoreResult};
use crate::llm::provider::{build_model, LlmSettings};

pub struct LlmService {
    model: Option<Arc<dyn LanguageModel>>,
    settings: LlmSettings,
}

impl LlmService {
    pub fn new() -> CoreResult<Self> {
        let settings = LlmSettings::from_env()?;
        Ok(Self {
            model: build_model(&settings).ok(),
            settings,
        })
    }

    pub async fn generate_reply_parts(
        &self,
        messages: &[llm_kit_provider_utils::message::Message],
        tools: llm_kit_core::tool::ToolSet,
    ) -> CoreResult<StreamTextResult> {
        let model = self.model.as_ref().ok_or_else(|| {
            CoreError::InvalidInput("missing LLM API key".to_string())
        })?;
        log::info!(
            "llm prompt messages count={}",
            messages.len(),
        );
        let prompt =
            Prompt::messages(messages.to_vec()).with_system(self.settings.system_prompt.clone());
        let result = StreamText::new(model.clone(), prompt)
            .temperature(self.settings.temperature)
            .max_output_tokens(self.settings.max_output_tokens)
            .tools(tools)
            .stop_when(vec![Box::new(step_count_is(self.settings.max_steps))])
            .execute()
            .await
            .map_err(|error| CoreError::Internal(error.to_string()))?;
        Ok(result)
    }
}
