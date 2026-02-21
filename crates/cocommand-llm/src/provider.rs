use async_trait::async_trait;

use crate::error::LlmError;
use crate::message::Message;
use crate::settings::LlmSettings;
use crate::stream::LlmStream;
use crate::tool::LlmToolSet;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn stream(
        &self,
        messages: &[Message],
        tools: LlmToolSet,
    ) -> Result<LlmStream, LlmError>;

    async fn update_settings(&self, settings: LlmSettings) -> Result<(), LlmError>;

    fn with_settings(&self, settings: LlmSettings) -> Result<Box<dyn LlmProvider>, LlmError>;
}
