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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::message::Message;
    use crate::workspace::WorkspaceInstance;
    use tempfile::tempdir;

    async fn setup() -> (
        tempfile::TempDir,
        Arc<WorkspaceInstance>,
        Bus,
        String,
        Message,
    ) {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::new(dir.path()).await.expect("workspace"));
        let bus = Bus::new(16);
        let session_id = "session_1".to_string();
        let assistant = Message::from_parts(&session_id, "assistant", Vec::new());
        MessageStorage::store_info(&workspace.storage, &assistant.info)
            .await
            .expect("store info");
        (dir, workspace, bus, session_id, assistant)
    }

    async fn load_assistant_parts(
        workspace: &WorkspaceInstance,
        session_id: &str,
        assistant_message_id: &str,
    ) -> Vec<MessagePart> {
        MessageStorage::load(&workspace.storage, session_id)
            .await
            .expect("load")
            .into_iter()
            .find(|message| message.info.id == assistant_message_id)
            .map(|message| message.parts)
            .unwrap_or_default()
    }

    #[tokio::test]
    async fn stores_text_part_and_reuses_part_id_for_same_segment() {
        let (_dir, workspace, bus, session_id, assistant) = setup().await;
        let run_id = "run_1".to_string();
        let context = StorePartContext::new(
            &workspace.storage,
            &bus,
            &run_id,
            &session_id,
            &assistant.info.id,
        );
        let mut processor = StreamProcessor::new();

        processor
            .on_part(
                LlmStreamEvent::TextDelta {
                    id: "text_1".to_string(),
                    text: "hello".to_string(),
                },
                &context,
            )
            .await
            .expect("first text delta");

        processor
            .on_part(
                LlmStreamEvent::TextDelta {
                    id: "text_1".to_string(),
                    text: " world".to_string(),
                },
                &context,
            )
            .await
            .expect("second text delta");

        processor
            .on_part(
                LlmStreamEvent::TextEnd {
                    id: "text_1".to_string(),
                },
                &context,
            )
            .await
            .expect("text end");

        let parts = load_assistant_parts(&workspace, &session_id, &assistant.info.id).await;
        assert_eq!(parts.len(), 1);
        assert!(matches!(
            parts.first(),
            Some(MessagePart::Text(TextPart { text, .. })) if text == "hello world"
        ));
    }

    #[tokio::test]
    async fn stores_distinct_text_parts_for_distinct_segments() {
        let (_dir, workspace, bus, session_id, assistant) = setup().await;
        let run_id = "run_1".to_string();
        let context = StorePartContext::new(
            &workspace.storage,
            &bus,
            &run_id,
            &session_id,
            &assistant.info.id,
        );
        let mut processor = StreamProcessor::new();

        processor
            .on_part(
                LlmStreamEvent::TextStart {
                    id: "text_1".to_string(),
                },
                &context,
            )
            .await
            .expect("text start 1");
        processor
            .on_part(
                LlmStreamEvent::TextDelta {
                    id: "text_1".to_string(),
                    text: "hello".to_string(),
                },
                &context,
            )
            .await
            .expect("text delta 1");
        processor
            .on_part(
                LlmStreamEvent::TextEnd {
                    id: "text_1".to_string(),
                },
                &context,
            )
            .await
            .expect("text end 1");

        processor
            .on_part(
                LlmStreamEvent::TextStart {
                    id: "text_2".to_string(),
                },
                &context,
            )
            .await
            .expect("text start 2");
        processor
            .on_part(
                LlmStreamEvent::TextDelta {
                    id: "text_2".to_string(),
                    text: "world".to_string(),
                },
                &context,
            )
            .await
            .expect("text delta 2");
        processor
            .on_part(
                LlmStreamEvent::TextEnd {
                    id: "text_2".to_string(),
                },
                &context,
            )
            .await
            .expect("text end 2");

        let parts = load_assistant_parts(&workspace, &session_id, &assistant.info.id).await;
        assert_eq!(parts.len(), 2);
        assert!(matches!(
            parts.first(),
            Some(MessagePart::Text(TextPart { text, .. })) if text == "hello"
        ));
        assert!(matches!(
            parts.get(1),
            Some(MessagePart::Text(TextPart { text, .. })) if text == "world"
        ));
    }

    #[tokio::test]
    async fn stores_tool_error_part() {
        let (_dir, workspace, bus, session_id, assistant) = setup().await;
        let run_id = "run_1".to_string();
        let context = StorePartContext::new(
            &workspace.storage,
            &bus,
            &run_id,
            &session_id,
            &assistant.info.id,
        );
        let mut processor = StreamProcessor::new();

        processor
            .on_part(
                LlmStreamEvent::ToolError {
                    tool_call_id: "call_1".to_string(),
                    tool_name: "test_tool".to_string(),
                    input: serde_json::json!({"input": true}),
                    error: serde_json::json!({"message": "failed"}),
                },
                &context,
            )
            .await
            .expect("tool error");

        let parts = load_assistant_parts(&workspace, &session_id, &assistant.info.id).await;
        assert_eq!(parts.len(), 1);
        assert!(matches!(
            parts.first(),
            Some(MessagePart::Tool(part))
                if part.call_id == "call_1"
                    && part.tool == "test_tool"
                    && matches!(
                        &part.state,
                        ToolState::Error(state) if state.error == "{\"message\":\"failed\"}"
                    )
        ));
    }

    #[tokio::test]
    async fn updates_tool_part_on_tool_result_in_place() {
        let (_dir, workspace, bus, session_id, assistant) = setup().await;
        let run_id = "run_1".to_string();
        let context = StorePartContext::new(
            &workspace.storage,
            &bus,
            &run_id,
            &session_id,
            &assistant.info.id,
        );
        let mut processor = StreamProcessor::new();

        processor
            .on_part(
                LlmStreamEvent::ToolCall {
                    tool_call_id: "call_1".to_string(),
                    tool_name: "test_tool".to_string(),
                    input: serde_json::json!({"input": true}),
                },
                &context,
            )
            .await
            .expect("tool call");

        let first_parts = load_assistant_parts(&workspace, &session_id, &assistant.info.id).await;
        let first_part_id = match first_parts.first() {
            Some(MessagePart::Tool(part)) => part.base.id.clone(),
            _ => panic!("expected tool part"),
        };

        processor
            .on_part(
                LlmStreamEvent::ToolResult {
                    tool_call_id: "call_1".to_string(),
                    tool_name: "test_tool".to_string(),
                    input: serde_json::json!({"input": true}),
                    output: serde_json::json!({"ok": true}),
                },
                &context,
            )
            .await
            .expect("tool result");

        let parts = load_assistant_parts(&workspace, &session_id, &assistant.info.id).await;
        assert_eq!(parts.len(), 1);
        assert!(matches!(
            parts.first(),
            Some(MessagePart::Tool(part))
                if part.base.id == first_part_id
                    && part.call_id == "call_1"
                    && part.tool == "test_tool"
                    && matches!(
                        &part.state,
                        ToolState::Completed(state) if state.output == "{\"ok\":true}"
                    )
        ));

        let stored_part_ids = workspace
            .storage
            .list(&["part", &assistant.info.id])
            .await
            .expect("list parts");
        assert_eq!(stored_part_ids.len(), 1);
        assert_eq!(stored_part_ids[0], first_part_id);
    }

    #[tokio::test]
    async fn returns_error_for_stream_error_part() {
        let (_dir, workspace, bus, session_id, assistant) = setup().await;
        let run_id = "run_1".to_string();
        let context = StorePartContext::new(
            &workspace.storage,
            &bus,
            &run_id,
            &session_id,
            &assistant.info.id,
        );
        let mut processor = StreamProcessor::new();

        let error = processor
            .on_part(
                LlmStreamEvent::Error {
                    error: serde_json::json!({"message": "boom"}),
                },
                &context,
            )
            .await
            .expect_err("stream error");

        assert!(error.to_string().contains("llm stream error"));
    }

    #[tokio::test]
    async fn ignores_meta_parts() {
        let (_dir, workspace, bus, session_id, assistant) = setup().await;
        let run_id = "run_1".to_string();
        let context = StorePartContext::new(
            &workspace.storage,
            &bus,
            &run_id,
            &session_id,
            &assistant.info.id,
        );
        let mut processor = StreamProcessor::new();

        processor
            .on_part(LlmStreamEvent::Start, &context)
            .await
            .expect("start");
        processor
            .on_part(LlmStreamEvent::Finish, &context)
            .await
            .expect("finish");

        let parts = load_assistant_parts(&workspace, &session_id, &assistant.info.id).await;
        assert!(parts.is_empty());
    }
}
