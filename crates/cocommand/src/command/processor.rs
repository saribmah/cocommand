use std::collections::HashMap;

use crate::bus::Bus;
use crate::command::session_message::SessionCommandPartUpdatedEvent;
use crate::error::{CoreError, CoreResult};
use crate::message::parts::{
    FilePart, MessagePart, PartBase, ReasoningPart, SourcePart, TextPart, ToolPart, ToolState,
    ToolStateCompleted, ToolStateError, ToolStateRunning, ToolStateTimeCompleted,
    ToolStateTimeRange, ToolStateTimeStart,
};
use crate::message::Message;
use crate::storage::SharedStorage;
use crate::utils::time::now_secs;
use llm_kit_core::stream_text::TextStreamPart;
use llm_kit_provider::language_model::content::source::LanguageModelSource;
use serde_json::{Map, Value};
use tokio_stream::StreamExt;

#[derive(Debug, Default, Clone)]
pub(crate) struct StreamProcessor {
    current_text_id: Option<String>,
    current_text_part_id: Option<String>,
    current_text: String,
    current_reasoning_id: Option<String>,
    current_reasoning: String,
    tool_calls: HashMap<String, ToolPart>,
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
                self.upsert_text_part(context).await?;
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
                let start = now_secs();
                let input = value_to_record(tool_call.input);
                let tool_part = ToolPart {
                    base: PartBase::new(context.session_id, context.message_id),
                    call_id: tool_call.tool_call_id,
                    tool: tool_call.tool_name,
                    state: ToolState::Running(ToolStateRunning {
                        input,
                        title: None,
                        metadata: None,
                        time: ToolStateTimeStart { start },
                    }),
                    metadata: None,
                };
                self.tool_calls
                    .insert(tool_part.call_id.clone(), tool_part.clone());
                self.store_part(MessagePart::Tool(tool_part), context)
                    .await?;
            }
            TextStreamPart::ToolResult { tool_result } => {
                let end = now_secs();
                let call_id = tool_result.tool_call_id;
                let tool = tool_result.tool_name;
                let fallback_input = value_to_record(tool_result.input);
                let output = value_to_string(&tool_result.output);
                let mut tool_part = self
                    .tool_calls
                    .remove(&call_id)
                    .unwrap_or_else(|| ToolPart {
                        base: PartBase::new(context.session_id, context.message_id),
                        call_id: call_id.clone(),
                        tool: tool.clone(),
                        state: ToolState::Running(ToolStateRunning {
                            input: fallback_input.clone(),
                            title: None,
                            metadata: None,
                            time: ToolStateTimeStart { start: end },
                        }),
                        metadata: None,
                    });
                let (input, start) = tool_state_input_and_start(&tool_part.state, end);
                tool_part.tool = tool.clone();
                tool_part.state = ToolState::Completed(ToolStateCompleted {
                    input: if input.is_empty() {
                        fallback_input
                    } else {
                        input
                    },
                    output,
                    title: tool,
                    metadata: Map::new(),
                    time: ToolStateTimeCompleted {
                        start,
                        end,
                        compacted: None,
                    },
                    attachments: None,
                });
                self.store_part(MessagePart::Tool(tool_part), context)
                    .await?;
            }
            TextStreamPart::ToolError { tool_error } => {
                let end = now_secs();
                let call_id = tool_error.tool_call_id;
                let tool = tool_error.tool_name;
                let fallback_input = value_to_record(tool_error.input);
                let error = value_to_string(&tool_error.error);
                let mut tool_part = self
                    .tool_calls
                    .remove(&call_id)
                    .unwrap_or_else(|| ToolPart {
                        base: PartBase::new(context.session_id, context.message_id),
                        call_id: call_id.clone(),
                        tool: tool.clone(),
                        state: ToolState::Running(ToolStateRunning {
                            input: fallback_input.clone(),
                            title: None,
                            metadata: None,
                            time: ToolStateTimeStart { start: end },
                        }),
                        metadata: None,
                    });
                let (input, start) = tool_state_input_and_start(&tool_part.state, end);
                tool_part.tool = tool;
                tool_part.state = ToolState::Error(ToolStateError {
                    input: if input.is_empty() {
                        fallback_input
                    } else {
                        input
                    },
                    error,
                    metadata: None,
                    time: ToolStateTimeRange { start, end },
                });
                self.store_part(MessagePart::Tool(tool_part), context)
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

    async fn flush_text(&mut self, _context: &StorePartContext<'_>) -> CoreResult<()> {
        self.current_text_id = None;
        self.current_text_part_id = None;
        self.current_text.clear();
        Ok(())
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
        if let Some(index) = self
            .mapped_parts
            .iter()
            .position(|existing| existing.base().id == part.base().id)
        {
            self.mapped_parts[index] = part;
        } else {
            self.mapped_parts.push(part);
        }
        Ok(())
    }

    async fn upsert_text_part(&mut self, context: &StorePartContext<'_>) -> CoreResult<()> {
        if self.current_text.is_empty() {
            return Ok(());
        }
        let part_id = if let Some(id) = &self.current_text_part_id {
            id.clone()
        } else {
            let base = PartBase::new(context.session_id, context.message_id);
            let id = base.id;
            self.current_text_part_id = Some(id.clone());
            id
        };
        self.store_part(
            MessagePart::Text(TextPart {
                base: PartBase {
                    id: part_id,
                    session_id: context.session_id.to_string(),
                    message_id: context.message_id.to_string(),
                },
                text: self.current_text.clone(),
            }),
            context,
        )
        .await
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

fn value_to_record(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(object) => object,
        other => {
            let mut wrapped = Map::new();
            wrapped.insert("value".to_string(), other);
            wrapped
        }
    }
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| "null".to_string()),
    }
}

fn tool_state_input_and_start(state: &ToolState, fallback_start: u64) -> (Map<String, Value>, u64) {
    match state {
        ToolState::Pending(state) => (state.input.clone(), fallback_start),
        ToolState::Running(state) => (state.input.clone(), state.time.start),
        ToolState::Completed(state) => (state.input.clone(), state.time.start),
        ToolState::Error(state) => (state.input.clone(), state.time.start),
    }
}
