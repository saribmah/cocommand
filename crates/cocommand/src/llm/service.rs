use std::sync::Arc;
use tokio::sync::RwLock;

use llm_kit_core::{prompt::Prompt, step_count_is, stream_text::StreamTextResult, StreamText};
use llm_kit_provider::LanguageModel;

use crate::error::{CoreError, CoreResult};
use crate::llm::provider::{build_model, LlmSettings};

struct LlmState {
    model: Option<Arc<dyn LanguageModel>>,
    settings: LlmSettings,
}

pub struct LlmService {
    state: Arc<RwLock<LlmState>>,
}

impl LlmService {
    pub fn new(settings: LlmSettings) -> CoreResult<Self> {
        Ok(Self {
            state: Arc::new(RwLock::new(LlmState {
                model: build_model(&settings).ok(),
                settings,
            })),
        })
    }

    pub async fn stream_text(
        &self,
        messages: &[llm_kit_provider_utils::message::Message],
        tools: llm_kit_core::tool::ToolSet,
    ) -> CoreResult<StreamTextResult> {
        let guard = self.state.read().await;
        let model = guard
            .model
            .as_ref()
            .ok_or_else(|| CoreError::InvalidInput("missing LLM API key".to_string()))?;
        tracing::info!("llm prompt messages count={}", messages.len(),);
        let prompt =
            Prompt::messages(messages.to_vec()).with_system(guard.settings.system_prompt.clone());
        let result = StreamText::new(model.clone(), prompt)
            .temperature(guard.settings.temperature)
            .max_output_tokens(guard.settings.max_output_tokens)
            .tools(tools)
            .stop_when(vec![Box::new(step_count_is(guard.settings.max_steps))])
            .execute()
            .await
            .map_err(|error| CoreError::Internal(error.to_string()))?;
        Ok(result)
    }

    pub async fn update_settings(&self, settings: LlmSettings) -> CoreResult<()> {
        let mut guard = self.state.write().await;
        guard.model = build_model(&settings).ok();
        guard.settings = settings;
        Ok(())
    }
}
