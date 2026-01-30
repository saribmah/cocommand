use std::sync::Arc;

use llm_kit_core::{StreamText, prompt::Prompt, step_count_is};
use llm_kit_provider::LanguageModel;

use crate::error::{CoreError, CoreResult};
use crate::llm::provider::{build_model, LlmSettings};
use crate::llm::tools::{build_tool_set, session_messages_to_prompt};
use crate::session::SessionContext;
use crate::workspace::WorkspaceInstance;

pub struct LlmService {
    workspace: Arc<WorkspaceInstance>,
    model: Arc<dyn LanguageModel>,
    settings: LlmSettings,
}

impl LlmService {
    pub fn new(workspace: Arc<WorkspaceInstance>) -> CoreResult<Self> {
        let settings = LlmSettings::from_env()?;
        let model = build_model(&settings)?;
        Ok(Self {
            workspace,
            model,
            settings,
        })
    }

    pub async fn generate_reply(&self, context: &SessionContext) -> CoreResult<String> {
        let messages = session_messages_to_prompt(&context.messages);
        log::info!(
            "llm prompt messages count={} session_id={}",
            messages.len(),
            context.session_id
        );
        for (index, message) in context.messages.iter().enumerate() {
            log::debug!(
                "llm prompt message {}: seq={} role={} chars={}",
                index,
                message.seq,
                message.role,
                message.text.len()
            );
        }
        let prompt = Prompt::messages(messages).with_system(self.settings.system_prompt.clone());
        let tools = build_tool_set(self.workspace.clone(), &context.session_id);

        let result = StreamText::new(self.model.clone(), prompt)
            .temperature(self.settings.temperature)
            .max_output_tokens(self.settings.max_output_tokens)
            .tools(tools)
            .stop_when(vec![Box::new(step_count_is(self.settings.max_steps))])
            .execute()
            .await
            .map_err(|error| CoreError::Internal(error.to_string()))?;
        let text = result
            .text()
            .await
            .map_err(|error| CoreError::Internal(error.to_string()))?;

        Ok(text)
    }
}
