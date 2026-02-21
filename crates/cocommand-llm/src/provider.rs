use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::error::LlmError;
use crate::message::Message;
use crate::settings::LlmSettings;
use crate::stream::LlmStream;
use crate::tool::LlmToolSet;

#[derive(Debug, Clone, Default)]
pub struct LlmStreamOptions {
    pub max_steps: Option<usize>,
    pub abort_signal: Option<CancellationToken>,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn stream(&self, messages: &[Message], tools: LlmToolSet) -> Result<LlmStream, LlmError>;

    async fn stream_with_options(
        &self,
        messages: &[Message],
        tools: LlmToolSet,
        options: LlmStreamOptions,
    ) -> Result<LlmStream, LlmError>;

    async fn update_settings(&self, settings: LlmSettings) -> Result<(), LlmError>;

    fn with_settings(&self, settings: LlmSettings) -> Result<Box<dyn LlmProvider>, LlmError>;
}
