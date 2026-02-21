use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;
use llm_kit_core::{prompt::Prompt, step_count_is, StreamText};
use llm_kit_openai_compatible::OpenAICompatibleClient;
use llm_kit_provider::LanguageModel;

use crate::error::LlmError;
use crate::kit::convert::messages_to_prompt;
use crate::kit::stream_map::map_kit_stream;
use crate::kit::tool_map::to_kit_tool_set;
use crate::message::Message;
use crate::provider::LlmProvider;
use crate::settings::LlmSettings;
use crate::stream::LlmStream;
use crate::tool::LlmToolSet;

struct KitState {
    model: Option<Arc<dyn LanguageModel>>,
    settings: LlmSettings,
}

pub struct LlmKitProvider {
    state: Arc<RwLock<KitState>>,
}

impl LlmKitProvider {
    pub fn new(settings: LlmSettings) -> Result<Self, LlmError> {
        Ok(Self {
            state: Arc::new(RwLock::new(KitState {
                model: build_model(&settings).ok(),
                settings,
            })),
        })
    }
}

#[async_trait]
impl LlmProvider for LlmKitProvider {
    async fn stream(
        &self,
        messages: &[Message],
        tools: LlmToolSet,
    ) -> Result<LlmStream, LlmError> {
        let guard = self.state.read().await;
        let model = guard
            .model
            .as_ref()
            .ok_or(LlmError::MissingApiKey)?;
        let prompt_messages = messages_to_prompt(messages);
        let prompt =
            Prompt::messages(prompt_messages).with_system(guard.settings.system_prompt.clone());
        let kit_tools = to_kit_tool_set(tools);
        let result = StreamText::new(model.clone(), prompt)
            .temperature(guard.settings.temperature)
            .max_output_tokens(guard.settings.max_output_tokens)
            .tools(kit_tools)
            .stop_when(vec![Box::new(step_count_is(guard.settings.max_steps))])
            .execute()
            .await
            .map_err(|error| LlmError::Internal(error.to_string()))?;
        Ok(map_kit_stream(result.full_stream()))
    }

    async fn update_settings(&self, settings: LlmSettings) -> Result<(), LlmError> {
        let mut guard = self.state.write().await;
        guard.model = build_model(&settings).ok();
        guard.settings = settings;
        Ok(())
    }

    fn with_settings(&self, settings: LlmSettings) -> Result<Box<dyn LlmProvider>, LlmError> {
        Ok(Box::new(LlmKitProvider::new(settings)?))
    }
}

fn build_model(settings: &LlmSettings) -> Result<Arc<dyn LanguageModel>, LlmError> {
    let api_key = settings
        .api_key
        .clone()
        .ok_or(LlmError::MissingApiKey)?;
    let provider = OpenAICompatibleClient::new()
        .base_url(&settings.base_url)
        .api_key(&api_key)
        .build();
    Ok(provider.chat_model(&settings.model))
}
