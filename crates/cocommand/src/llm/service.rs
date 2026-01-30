use std::sync::Arc;

use llm_kit_core::{StreamText, prompt::Prompt, step_count_is};
use llm_kit_provider::LanguageModel;

use crate::error::{CoreError, CoreResult};
use crate::llm::provider::{build_model, LlmSettings};
use crate::llm::tools::messages_to_prompt;
use crate::message::{MessagePart, MessageWithParts};

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
        messages: &[MessageWithParts],
        tools: llm_kit_core::tool::ToolSet,
    ) -> CoreResult<Vec<MessagePart>> {
        let model = self.model.as_ref().ok_or_else(|| {
            CoreError::InvalidInput("missing LLM API key".to_string())
        })?;
        let prompt_messages = messages_to_prompt(messages);
        log::info!(
            "llm prompt messages count={}",
            prompt_messages.len(),
        );
        for (index, message) in messages.iter().enumerate() {
            log::debug!(
                "llm prompt message {}: id={} role={} parts={}",
                index,
                message.info.id,
                message.info.role,
                message.parts.len()
            );
        }
        let prompt = Prompt::messages(prompt_messages).with_system(self.settings.system_prompt.clone());
        let result = StreamText::new(model.clone(), prompt)
            .temperature(self.settings.temperature)
            .max_output_tokens(self.settings.max_output_tokens)
            .tools(tools)
            .stop_when(vec![Box::new(step_count_is(self.settings.max_steps))])
            .execute()
            .await
            .map_err(|error| CoreError::Internal(error.to_string()))?;
        let parts = crate::message::stream_result_to_parts(&result).await?;
        Ok(parts)
    }
}
