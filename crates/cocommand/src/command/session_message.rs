use std::sync::Arc;

use crate::bus::Bus;
use crate::error::{CoreError, CoreResult};
use crate::llm::LlmService;
use crate::message::parts::{
    FilePart, MessagePart, PartBase, ReasoningPart, SourcePart, TextPart, ToolCallPart,
    ToolErrorPart, ToolResultPart,
};
use crate::message::{Message, MessageInfo};
use crate::session::{SessionContext, SessionManager};
use crate::storage::SharedStorage;
use crate::tool::ToolRegistry;
use crate::workspace::WorkspaceInstance;
use llm_kit_core::stream_text::TextStreamPart;
use llm_kit_provider::language_model::content::source::LanguageModelSource;
use serde::Serialize;
use tokio_stream::StreamExt;

#[derive(Debug, Clone)]
pub struct SessionMessageCommandInput {
    pub request_id: String,
    pub text: String,
}

pub struct SessionMessageCommandOutput {
    pub context: SessionContext,
    pub user_message: MessageInfo,
    pub assistant_message: MessageInfo,
    pub reply_parts: Vec<MessagePart>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionMessagePartUpdatedEvent {
    pub event: String,
    pub request_id: String,
    pub session_id: String,
    pub message_id: String,
    pub part_id: String,
    pub part: MessagePart,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionMessageContextEvent {
    pub event: String,
    pub request_id: String,
    pub context: SessionContext,
}

struct PreparedSessionMessage {
    context: SessionContext,
    active_extension_ids: Vec<String>,
    user_message: MessageInfo,
    prompt_messages: Vec<llm_kit_provider_utils::message::Message>,
}

#[derive(Debug, Default, Clone)]
pub struct SessionMessageStreamState {
    current_text_id: Option<String>,
    current_text: String,
    current_reasoning_id: Option<String>,
    current_reasoning: String,
    mapped_parts: Vec<MessagePart>,
}

struct StorePartContext<'a> {
    storage: &'a SharedStorage,
    bus: &'a Bus,
    request_id: &'a str,
    session_id: &'a str,
    message_id: &'a str,
}

impl SessionMessageStreamState {
    pub fn new() -> Self {
        Self::default()
    }

    async fn on_part(
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

    async fn process<S>(&mut self, mut stream: S, context: &StorePartContext<'_>) -> CoreResult<()>
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

    pub fn mapped_parts(&self) -> &[MessagePart] {
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
        let _ = context.bus.publish(SessionMessagePartUpdatedEvent {
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

pub async fn run_session_message_command(
    sessions: Arc<SessionManager>,
    workspace: WorkspaceInstance,
    llm: &LlmService,
    bus: &Bus,
    input: SessionMessageCommandInput,
) -> CoreResult<SessionMessageCommandOutput> {
    let request_id = input.request_id.clone();
    let prepared =
        match prepare_session_message(sessions.clone(), workspace.clone(), input.text).await {
            Ok(prepared) => prepared,
            Err(error) => return Err(error),
        };
    let storage = workspace.storage.clone();
    let session_id = prepared.context.session_id.clone();

    publish_context(bus, &request_id, &prepared.context);

    let mut assistant_message = Message::from_parts(&session_id, "assistant", Vec::new());
    if let Err(error) = Message::store_info(&storage, &assistant_message.info).await {
        return Err(error);
    }

    let tools = ToolRegistry::tools(
        Arc::new(workspace),
        sessions,
        &prepared.context.session_id,
        &prepared.active_extension_ids,
    )
    .await;
    let reply = match llm.stream_text(&prepared.prompt_messages, tools).await {
        Ok(reply) => reply,
        Err(error) => return Err(error),
    };

    let mut stream_state = SessionMessageStreamState::new();
    let store_context = StorePartContext {
        storage: &storage,
        bus,
        request_id: &request_id,
        session_id: &session_id,
        message_id: &assistant_message.info.id,
    };
    if let Err(error) = stream_state
        .process(reply.full_stream(), &store_context)
        .await
    {
        return Err(error);
    }

    if let Err(error) = Message::touch_info(&storage, &mut assistant_message.info).await {
        return Err(error);
    }

    let reply_parts = stream_state.mapped_parts().to_vec();
    Ok(SessionMessageCommandOutput {
        context: prepared.context,
        user_message: prepared.user_message,
        assistant_message: assistant_message.info,
        reply_parts,
    })
}

async fn prepare_session_message(
    sessions: Arc<SessionManager>,
    workspace: WorkspaceInstance,
    text: String,
) -> CoreResult<PreparedSessionMessage> {
    let storage = workspace.storage.clone();
    let (active_extension_ids, context) = sessions
        .with_session_mut(|session| {
            Box::pin(async move {
                let active_extension_ids = session.active_extension_ids();
                let context = session.context(None).await?;
                Ok((active_extension_ids, context))
            })
        })
        .await?;

    let user_message = Message::from_text(&context.session_id, "user", &text);
    Message::store(&storage, &user_message).await?;

    let message_history = Message::load(&storage, &context.session_id).await?;
    let prompt_messages = message_history
        .iter()
        .flat_map(Message::to_prompt_messages)
        .collect();

    Ok(PreparedSessionMessage {
        context,
        active_extension_ids,
        user_message: user_message.info,
        prompt_messages,
    })
}

fn publish_context(bus: &Bus, request_id: &str, context: &SessionContext) {
    let _ = bus.publish(SessionMessageContextEvent {
        event: "context".to_string(),
        request_id: request_id.to_string(),
        context: context.clone(),
    });
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    use crate::bus::Bus;
    use crate::message::MessagePart;
    use crate::session::SessionContext;
    use crate::session::SessionManager;
    use crate::workspace::WorkspaceInstance;
    use llm_kit_provider_utils::tool::ToolError;

    #[tokio::test]
    async fn prepare_session_message_stores_user_message() {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::new(dir.path()).await.expect("workspace"));
        let sessions = Arc::new(SessionManager::new(workspace.clone()));

        let output =
            prepare_session_message(sessions, workspace.as_ref().clone(), "hello".to_string())
                .await
                .expect("command");

        assert_eq!(output.user_message.role, "user");
        assert_eq!(output.context.session_id, output.user_message.session_id);

        let messages = Message::load(&workspace.storage, &output.context.session_id)
            .await
            .expect("load");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].info.id, output.user_message.id);
        assert!(matches!(
            messages[0].parts.first(),
            Some(MessagePart::Text(part)) if part.text == "hello"
        ));
        assert!(!output.prompt_messages.is_empty());
    }

    #[tokio::test]
    async fn run_session_message_command_returns_error_when_llm_missing_key() {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::new(dir.path()).await.expect("workspace"));
        let sessions = Arc::new(SessionManager::new(workspace.clone()));
        let settings = {
            let config = workspace.config.read().await;
            crate::llm::LlmSettings::from_workspace(&config.llm)
        };
        let llm = LlmService::new(settings).expect("llm");
        let bus = Bus::new(32);

        let session_id = current_session_context(sessions.clone()).await.session_id;
        let result = run_session_message_command(
            sessions,
            workspace.as_ref().clone(),
            &llm,
            &bus,
            SessionMessageCommandInput {
                request_id: "request-1".to_string(),
                text: "hello".to_string(),
            },
        )
        .await;

        assert!(result.is_err());
        let messages = Message::load(&workspace.storage, &session_id)
            .await
            .expect("load");
        assert_eq!(messages.len(), 2);
        assert!(matches!(
            messages[0].parts.first(),
            Some(MessagePart::Text(part)) if part.text == "hello"
        ));
        assert_eq!(messages[1].info.role, "assistant");
        assert!(messages[1].parts.is_empty());
    }

    async fn current_session_context(sessions: Arc<SessionManager>) -> SessionContext {
        sessions
            .with_session_mut(|session| Box::pin(async move { session.context(None).await }))
            .await
            .expect("session context")
    }

    #[tokio::test]
    async fn stream_state_maps_text_deltas_to_part_on_text_end() {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::new(dir.path()).await.expect("workspace"));
        let bus = Bus::new(16);
        let request_id = "request_1".to_string();
        let session_id = "session_1".to_string();
        let assistant = Message::from_parts(&session_id, "assistant", Vec::new());
        Message::store_info(&workspace.storage, &assistant.info)
            .await
            .expect("store info");

        let context = StorePartContext {
            storage: &workspace.storage,
            bus: &bus,
            request_id: &request_id,
            session_id: &session_id,
            message_id: &assistant.info.id,
        };
        let mut state = SessionMessageStreamState::new();

        state
            .on_part(
                TextStreamPart::TextDelta {
                    id: "text_1".to_string(),
                    provider_metadata: None,
                    text: "hello".to_string(),
                },
                &context,
            )
            .await
            .expect("text delta");

        state
            .on_part(
                TextStreamPart::TextEnd {
                    id: "text_1".to_string(),
                    provider_metadata: None,
                },
                &context,
            )
            .await
            .expect("text end");

        assert_eq!(state.mapped_parts().len(), 1);
        assert!(matches!(
            state.mapped_parts().first(),
            Some(MessagePart::Text(TextPart { text, .. })) if text == "hello"
        ));

        let stored = Message::load(&workspace.storage, &session_id)
            .await
            .expect("load");
        let assistant = stored
            .iter()
            .find(|message| message.info.id == assistant.info.id)
            .expect("assistant");
        assert_eq!(assistant.parts.len(), 1);
        assert!(matches!(
            assistant.parts.first(),
            Some(MessagePart::Text(TextPart { text, .. })) if text == "hello"
        ));
    }

    #[tokio::test]
    async fn stream_state_creates_distinct_text_parts_per_text_end() {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::new(dir.path()).await.expect("workspace"));
        let bus = Bus::new(16);
        let request_id = "request_1".to_string();
        let session_id = "session_1".to_string();
        let assistant = Message::from_parts(&session_id, "assistant", Vec::new());
        Message::store_info(&workspace.storage, &assistant.info)
            .await
            .expect("store info");

        let context = StorePartContext {
            storage: &workspace.storage,
            bus: &bus,
            request_id: &request_id,
            session_id: &session_id,
            message_id: &assistant.info.id,
        };
        let mut state = SessionMessageStreamState::new();

        state
            .on_part(
                TextStreamPart::TextStart {
                    id: "text_1".to_string(),
                    provider_metadata: None,
                },
                &context,
            )
            .await
            .expect("text start 1");
        state
            .on_part(
                TextStreamPart::TextDelta {
                    id: "text_1".to_string(),
                    provider_metadata: None,
                    text: "hello".to_string(),
                },
                &context,
            )
            .await
            .expect("text delta 1");
        state
            .on_part(
                TextStreamPart::TextEnd {
                    id: "text_1".to_string(),
                    provider_metadata: None,
                },
                &context,
            )
            .await
            .expect("text end 1");

        state
            .on_part(
                TextStreamPart::TextStart {
                    id: "text_2".to_string(),
                    provider_metadata: None,
                },
                &context,
            )
            .await
            .expect("text start 2");
        state
            .on_part(
                TextStreamPart::TextDelta {
                    id: "text_2".to_string(),
                    provider_metadata: None,
                    text: "world".to_string(),
                },
                &context,
            )
            .await
            .expect("text delta 2");
        state
            .on_part(
                TextStreamPart::TextEnd {
                    id: "text_2".to_string(),
                    provider_metadata: None,
                },
                &context,
            )
            .await
            .expect("text end 2");

        assert_eq!(state.mapped_parts().len(), 2);
        assert!(matches!(
            state.mapped_parts().first(),
            Some(MessagePart::Text(TextPart { text, .. })) if text == "hello"
        ));
        assert!(matches!(
            state.mapped_parts().get(1),
            Some(MessagePart::Text(TextPart { text, .. })) if text == "world"
        ));
    }

    #[tokio::test]
    async fn stream_state_maps_tool_error_to_error_tool_result_part() {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::new(dir.path()).await.expect("workspace"));
        let bus = Bus::new(16);
        let request_id = "request_1".to_string();
        let session_id = "session_1".to_string();
        let assistant = Message::from_parts(&session_id, "assistant", Vec::new());
        Message::store_info(&workspace.storage, &assistant.info)
            .await
            .expect("store info");

        let context = StorePartContext {
            storage: &workspace.storage,
            bus: &bus,
            request_id: &request_id,
            session_id: &session_id,
            message_id: &assistant.info.id,
        };
        let mut state = SessionMessageStreamState::new();

        state
            .on_part(
                TextStreamPart::ToolError {
                    tool_error: ToolError::new(
                        "call_1",
                        "test_tool",
                        serde_json::json!({"input": true}),
                        serde_json::json!({"message": "failed"}),
                    ),
                },
                &context,
            )
            .await
            .expect("tool error");

        assert!(matches!(
            state.mapped_parts().first(),
            Some(MessagePart::ToolError(part))
            if part.call_id == "call_1"
                && part.tool_name == "test_tool"
                && part.error == serde_json::json!({"message": "failed"})
        ));
    }

    #[tokio::test]
    async fn stream_state_process_flushes_remaining_buffers() {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::new(dir.path()).await.expect("workspace"));
        let bus = Bus::new(16);
        let request_id = "request_1".to_string();
        let session_id = "session_1".to_string();
        let assistant = Message::from_parts(&session_id, "assistant", Vec::new());
        Message::store_info(&workspace.storage, &assistant.info)
            .await
            .expect("store info");

        let context = StorePartContext {
            storage: &workspace.storage,
            bus: &bus,
            request_id: &request_id,
            session_id: &session_id,
            message_id: &assistant.info.id,
        };
        let mut state = SessionMessageStreamState::new();
        let stream = tokio_stream::iter(vec![
            TextStreamPart::TextDelta {
                id: "text_1".to_string(),
                provider_metadata: None,
                text: "hello".to_string(),
            },
            TextStreamPart::ReasoningDelta {
                id: "reasoning_1".to_string(),
                provider_metadata: None,
                text: "think".to_string(),
            },
        ]);

        state.process(stream, &context).await.expect("process");

        assert!(state.mapped_parts().iter().any(|part| matches!(
            part,
            MessagePart::Text(TextPart { text, .. }) if text == "hello"
        )));
        assert!(state.mapped_parts().iter().any(|part| matches!(
            part,
            MessagePart::Reasoning(ReasoningPart { text, .. }) if text == "think"
        )));
    }

    #[tokio::test]
    async fn stream_state_returns_error_for_stream_error_part() {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::new(dir.path()).await.expect("workspace"));
        let bus = Bus::new(16);
        let request_id = "request_1".to_string();
        let session_id = "session_1".to_string();
        let assistant = Message::from_parts(&session_id, "assistant", Vec::new());
        Message::store_info(&workspace.storage, &assistant.info)
            .await
            .expect("store info");

        let context = StorePartContext {
            storage: &workspace.storage,
            bus: &bus,
            request_id: &request_id,
            session_id: &session_id,
            message_id: &assistant.info.id,
        };
        let mut state = SessionMessageStreamState::new();
        let error = state
            .on_part(
                TextStreamPart::Error {
                    error: serde_json::json!({"message": "boom"}),
                },
                &context,
            )
            .await
            .expect_err("stream error");

        assert!(error.to_string().contains("llm stream error"));
    }

    #[tokio::test]
    async fn stream_state_ignores_meta_parts() {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::new(dir.path()).await.expect("workspace"));
        let bus = Bus::new(16);
        let request_id = "request_1".to_string();
        let session_id = "session_1".to_string();
        let assistant = Message::from_parts(&session_id, "assistant", Vec::new());
        Message::store_info(&workspace.storage, &assistant.info)
            .await
            .expect("store info");

        let context = StorePartContext {
            storage: &workspace.storage,
            bus: &bus,
            request_id: &request_id,
            session_id: &session_id,
            message_id: &assistant.info.id,
        };
        let mut state = SessionMessageStreamState::new();
        state
            .on_part(TextStreamPart::Start, &context)
            .await
            .expect("start");

        assert!(state.mapped_parts().is_empty());
    }
}
