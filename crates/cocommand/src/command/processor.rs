use crate::bus::Bus;
use crate::command::session_message::SessionCommandPartUpdatedEvent;
use crate::error::{CoreError, CoreResult};
use crate::message::parts::{
    FilePart, MessagePart, PartBase, ReasoningPart, SourcePart, TextPart, ToolCallPart,
    ToolErrorPart, ToolResultPart,
};
use crate::message::Message;
use crate::storage::SharedStorage;
use llm_kit_core::stream_text::TextStreamPart;
use llm_kit_provider::language_model::content::source::LanguageModelSource;
use tokio_stream::StreamExt;

#[derive(Debug, Default, Clone)]
pub(crate) struct StreamProcessor {
    current_text_id: Option<String>,
    current_text: String,
    current_reasoning_id: Option<String>,
    current_reasoning: String,
    mapped_parts: Vec<MessagePart>,
}

pub(crate) struct StorePartContext<'a> {
    storage: &'a SharedStorage,
    bus: &'a Bus,
    request_id: &'a str,
    session_id: &'a str,
    message_id: &'a str,
}

impl<'a> StorePartContext<'a> {
    pub(crate) fn new(
        storage: &'a SharedStorage,
        bus: &'a Bus,
        request_id: &'a str,
        session_id: &'a str,
        message_id: &'a str,
    ) -> Self {
        Self {
            storage,
            bus,
            request_id,
            session_id,
            message_id,
        }
    }
}

impl StreamProcessor {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) async fn on_part(
        &mut self,
        part: TextStreamPart,
        context: &StorePartContext<'_>,
    ) -> CoreResult<()> {
        match part {
            TextStreamPart::TextStart { id, .. } => {
                self.flush_text(context).await?;
                self.current_text_id = Some(id);
            }
            TextStreamPart::TextDelta { id, text, .. } => {
                self.switch_text_segment_if_needed(id, context).await?;
                self.current_text.push_str(&text);
            }
            TextStreamPart::TextEnd { id, .. } => {
                self.switch_text_segment_if_needed(id, context).await?;
                self.flush_text(context).await?;
            }
            TextStreamPart::ReasoningStart { id, .. } => {
                self.flush_reasoning(context).await?;
                self.current_reasoning_id = Some(id);
            }
            TextStreamPart::ReasoningDelta { id, text, .. } => {
                self.switch_reasoning_segment_if_needed(id, context).await?;
                self.current_reasoning.push_str(&text);
            }
            TextStreamPart::ReasoningEnd { id, .. } => {
                self.switch_reasoning_segment_if_needed(id, context).await?;
                self.flush_reasoning(context).await?;
            }
            TextStreamPart::ToolInputStart { .. } => {}
            TextStreamPart::ToolInputDelta { .. } => {}
            TextStreamPart::ToolInputEnd { .. } => {}
            TextStreamPart::Source { source } => {
                self.store_part(
                    MessagePart::Source(map_source_to_part(
                        &source,
                        context.session_id,
                        context.message_id,
                    )),
                    context,
                )
                .await?;
            }
            TextStreamPart::File { file } => {
                self.store_part(
                    MessagePart::File(FilePart {
                        base: PartBase::new(context.session_id, context.message_id),
                        base64: file.base64,
                        media_type: file.media_type,
                        name: file.name,
                    }),
                    context,
                )
                .await?;
            }
            TextStreamPart::ToolCall { tool_call } => {
                self.store_part(
                    MessagePart::ToolCall(ToolCallPart {
                        base: PartBase::new(context.session_id, context.message_id),
                        call_id: tool_call.tool_call_id,
                        tool_name: tool_call.tool_name,
                        input: tool_call.input,
                    }),
                    context,
                )
                .await?;
            }
            TextStreamPart::ToolResult { tool_result } => {
                self.store_part(
                    MessagePart::ToolResult(ToolResultPart {
                        base: PartBase::new(context.session_id, context.message_id),
                        call_id: tool_result.tool_call_id,
                        tool_name: tool_result.tool_name,
                        output: tool_result.output,
                        is_error: false,
                    }),
                    context,
                )
                .await?;
            }
            TextStreamPart::ToolError { tool_error } => {
                self.store_part(
                    MessagePart::ToolError(ToolErrorPart {
                        base: PartBase::new(context.session_id, context.message_id),
                        call_id: tool_error.tool_call_id,
                        tool_name: tool_error.tool_name,
                        error: tool_error.error,
                    }),
                    context,
                )
                .await?;
            }
            TextStreamPart::ToolOutputDenied { .. } => {}
            TextStreamPart::ToolApprovalRequest { .. } => {}
            TextStreamPart::StartStep { .. } => {}
            TextStreamPart::FinishStep { .. } => {}
            TextStreamPart::Start => {}
            TextStreamPart::Finish { .. } => {}
            TextStreamPart::Abort => {}
            TextStreamPart::Error { error } => {
                return Err(CoreError::Internal(format!("llm stream error: {error}")));
            }
            TextStreamPart::Raw { .. } => {}
        }

        Ok(())
    }

    pub(crate) async fn process<S>(
        &mut self,
        mut stream: S,
        context: &StorePartContext<'_>,
    ) -> CoreResult<()>
    where
        S: tokio_stream::Stream<Item = TextStreamPart> + Unpin,
    {
        while let Some(part) = stream.next().await {
            self.on_part(part, context).await?;
        }
        self.flush_text(context).await?;
        self.flush_reasoning(context).await?;
        Ok(())
    }

    pub(crate) fn mapped_parts(&self) -> &[MessagePart] {
        &self.mapped_parts
    }

    async fn flush_text(&mut self, context: &StorePartContext<'_>) -> CoreResult<()> {
        self.current_text_id = None;
        if self.current_text.is_empty() {
            return Ok(());
        }
        let text = std::mem::take(&mut self.current_text);
        self.store_part(
            MessagePart::Text(TextPart {
                base: PartBase::new(context.session_id, context.message_id),
                text,
            }),
            context,
        )
        .await
    }

    async fn flush_reasoning(&mut self, context: &StorePartContext<'_>) -> CoreResult<()> {
        self.current_reasoning_id = None;
        if self.current_reasoning.is_empty() {
            return Ok(());
        }
        let text = std::mem::take(&mut self.current_reasoning);
        self.store_part(
            MessagePart::Reasoning(ReasoningPart {
                base: PartBase::new(context.session_id, context.message_id),
                text,
            }),
            context,
        )
        .await
    }

    async fn store_part(
        &mut self,
        part: MessagePart,
        context: &StorePartContext<'_>,
    ) -> CoreResult<()> {
        let part_id = part.base().id.clone();
        Message::store_part(context.storage, &part).await?;
        let _ = context.bus.publish(SessionCommandPartUpdatedEvent {
            event: "part.updated".to_string(),
            request_id: context.request_id.to_string(),
            session_id: context.session_id.to_string(),
            message_id: context.message_id.to_string(),
            part_id,
            part: part.clone(),
        });
        self.mapped_parts.push(part);
        Ok(())
    }

    async fn switch_text_segment_if_needed(
        &mut self,
        id: String,
        context: &StorePartContext<'_>,
    ) -> CoreResult<()> {
        if self.current_text_id.as_deref() == Some(id.as_str()) {
            return Ok(());
        }
        self.flush_text(context).await?;
        self.current_text_id = Some(id);
        Ok(())
    }

    async fn switch_reasoning_segment_if_needed(
        &mut self,
        id: String,
        context: &StorePartContext<'_>,
    ) -> CoreResult<()> {
        if self.current_reasoning_id.as_deref() == Some(id.as_str()) {
            return Ok(());
        }
        self.flush_reasoning(context).await?;
        self.current_reasoning_id = Some(id);
        Ok(())
    }
}

fn map_source_to_part(
    source: &llm_kit_core::output::SourceOutput,
    session_id: &str,
    message_id: &str,
) -> SourcePart {
    match &source.source {
        LanguageModelSource::Url { id, url, title, .. } => SourcePart {
            base: PartBase::new(session_id, message_id),
            source_id: Some(id.clone()),
            source_type: "url".to_string(),
            url: Some(url.clone()),
            title: title.clone(),
            media_type: None,
            filename: None,
        },
        LanguageModelSource::Document {
            id,
            media_type,
            title,
            filename,
            ..
        } => SourcePart {
            base: PartBase::new(session_id, message_id),
            source_id: Some(id.clone()),
            source_type: "document".to_string(),
            url: None,
            title: Some(title.clone()),
            media_type: Some(media_type.clone()),
            filename: filename.clone(),
        },
    }
}
