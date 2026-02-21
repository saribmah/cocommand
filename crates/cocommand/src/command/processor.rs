use std::collections::HashMap;

use crate::bus::Bus;
use crate::error::{CoreError, CoreResult};
use crate::event::{CoreEvent, SessionPartUpdatedPayload};
use crate::message::message::MessageStorage;
use crate::message::parts::{
    FilePart, MessagePart, PartBase, ReasoningPart, TextPart, ToolPart, ToolState,
    ToolStateCompleted, ToolStateError, ToolStateRunning, ToolStateTimeCompleted,
    ToolStateTimeRange, ToolStateTimeStart,
};
use crate::storage::SharedStorage;
use crate::utils::time::now_secs;
use cocommand_llm::LlmStreamEvent;
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
    run_id: &'a str,
    session_id: &'a str,
    message_id: &'a str,
}

impl<'a> StorePartContext<'a> {
    pub(crate) fn new(
        storage: &'a SharedStorage,
        bus: &'a Bus,
        run_id: &'a str,
        session_id: &'a str,
        message_id: &'a str,
    ) -> Self {
        Self {
            storage,
            bus,
            run_id,
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
        part: LlmStreamEvent,
        context: &StorePartContext<'_>,
    ) -> CoreResult<()> {
        match part {
            LlmStreamEvent::TextStart { id } => {
                self.flush_text(context).await?;
                self.current_text_id = Some(id);
            }
            LlmStreamEvent::TextDelta { id, text } => {
                self.switch_text_segment_if_needed(id, context).await?;
                self.current_text.push_str(&text);
                self.upsert_text_part(context).await?;
            }
            LlmStreamEvent::TextEnd { id } => {
                self.switch_text_segment_if_needed(id, context).await?;
                self.flush_text(context).await?;
            }
            LlmStreamEvent::ReasoningStart { id } => {
                self.flush_reasoning(context).await?;
                self.current_reasoning_id = Some(id);
            }
            LlmStreamEvent::ReasoningDelta { id, text } => {
                self.switch_reasoning_segment_if_needed(id, context).await?;
                self.current_reasoning.push_str(&text);
            }
            LlmStreamEvent::ReasoningEnd { id } => {
                self.switch_reasoning_segment_if_needed(id, context).await?;
                self.flush_reasoning(context).await?;
            }
            LlmStreamEvent::File {
                base64,
                media_type,
                name,
            } => {
                self.store_part(
                    MessagePart::File(FilePart {
                        base: PartBase::new(context.session_id, context.message_id),
                        base64,
                        media_type,
                        name,
                        source: None,
                    }),
                    context,
                )
                .await?;
            }
            LlmStreamEvent::ToolCall {
                tool_call_id,
                tool_name,
                input,
            } => {
                let start = now_secs();
                let input_map = value_to_record(input);
                let tool_part = ToolPart {
                    base: PartBase::new(context.session_id, context.message_id),
                    call_id: tool_call_id,
                    tool: tool_name,
                    state: ToolState::Running(ToolStateRunning {
                        input: input_map,
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
            LlmStreamEvent::ToolResult {
                tool_call_id,
                tool_name,
                input,
                output,
            } => {
                let end = now_secs();
                let fallback_input = value_to_record(input);
                let output_str = value_to_string(&output);
                let mut tool_part =
                    self.tool_calls
                        .remove(&tool_call_id)
                        .unwrap_or_else(|| ToolPart {
                            base: PartBase::new(context.session_id, context.message_id),
                            call_id: tool_call_id.clone(),
                            tool: tool_name.clone(),
                            state: ToolState::Running(ToolStateRunning {
                                input: fallback_input.clone(),
                                title: None,
                                metadata: None,
                                time: ToolStateTimeStart { start: end },
                            }),
                            metadata: None,
                        });
                let (prev_input, start) = tool_state_input_and_start(&tool_part.state, end);
                tool_part.tool = tool_name.clone();
                tool_part.state = ToolState::Completed(ToolStateCompleted {
                    input: if prev_input.is_empty() {
                        fallback_input
                    } else {
                        prev_input
                    },
                    output: output_str,
                    title: tool_name,
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
            LlmStreamEvent::ToolError {
                tool_call_id,
                tool_name,
                input,
                error,
            } => {
                let end = now_secs();
                let fallback_input = value_to_record(input);
                let error_str = value_to_string(&error);
                let mut tool_part =
                    self.tool_calls
                        .remove(&tool_call_id)
                        .unwrap_or_else(|| ToolPart {
                            base: PartBase::new(context.session_id, context.message_id),
                            call_id: tool_call_id.clone(),
                            tool: tool_name.clone(),
                            state: ToolState::Running(ToolStateRunning {
                                input: fallback_input.clone(),
                                title: None,
                                metadata: None,
                                time: ToolStateTimeStart { start: end },
                            }),
                            metadata: None,
                        });
                let (prev_input, start) = tool_state_input_and_start(&tool_part.state, end);
                tool_part.tool = tool_name;
                tool_part.state = ToolState::Error(ToolStateError {
                    input: if prev_input.is_empty() {
                        fallback_input
                    } else {
                        prev_input
                    },
                    error: error_str,
                    metadata: None,
                    time: ToolStateTimeRange { start, end },
                });
                self.store_part(MessagePart::Tool(tool_part), context)
                    .await?;
            }
            LlmStreamEvent::Error { error } => {
                return Err(CoreError::Internal(format!("llm stream error: {error}")));
            }
            LlmStreamEvent::Start | LlmStreamEvent::Finish => {}
        }

        Ok(())
    }

    pub(crate) async fn process<S>(
        &mut self,
        mut stream: S,
        context: &StorePartContext<'_>,
    ) -> CoreResult<()>
    where
        S: tokio_stream::Stream<Item = LlmStreamEvent> + Unpin,
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

    pub(crate) fn tool_call(&self, tool_call_id: &str) -> Option<ToolPart> {
        self.tool_calls.get(tool_call_id).cloned()
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
        MessageStorage::store_part(context.storage, &part).await?;
        let _ = context
            .bus
            .publish(CoreEvent::SessionPartUpdated(SessionPartUpdatedPayload {
                session_id: context.session_id.to_string(),
                run_id: context.run_id.to_string(),
                message_id: context.message_id.to_string(),
                part_id,
                part: part.clone(),
            }));
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
