use std::sync::Arc;

use crate::bus::Bus;
use crate::command::processor::{StorePartContext, StreamProcessor};
use crate::error::CoreResult;
use crate::llm::LlmService;
use crate::message::{
    ExtensionPart, FilePartSourceText, Message, MessageInfo, MessagePart, PartBase, TextPart,
};
use crate::session::{SessionContext, SessionManager};
use crate::tool::ToolRegistry;
use crate::workspace::WorkspaceInstance;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct SessionCommandInput {
    pub request_id: String,
    pub parts: Vec<SessionCommandInputPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SessionCommandInputPart {
    Text(SessionCommandTextPartInput),
    Extension(SessionCommandExtensionPartInput),
    File(SessionCommandFilePartInput),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionCommandTextPartInput {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionCommandExtensionPartInput {
    #[serde(rename = "extensionId")]
    pub extension_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<FilePartSourceText>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionCommandFilePartInput {
    pub path: String,
    pub name: String,
    #[serde(rename = "entryType", skip_serializing_if = "Option::is_none")]
    pub entry_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<FilePartSourceText>,
}

pub struct SessionCommandOutput {
    pub context: SessionContext,
    pub user_message: MessageInfo,
    pub assistant_message: MessageInfo,
    pub reply_parts: Vec<MessagePart>,
}

struct PreparedSessionCommand {
    context: SessionContext,
    active_extension_ids: Vec<String>,
    user_message: MessageInfo,
    prompt_messages: Vec<llm_kit_provider_utils::message::Message>,
}

pub async fn run_session_command(
    sessions: Arc<SessionManager>,
    workspace: WorkspaceInstance,
    llm: &LlmService,
    bus: &Bus,
    input: SessionCommandInput,
) -> CoreResult<SessionCommandOutput> {
    let request_id = input.request_id.clone();
    let prepared =
        match prepare_session_message(sessions.clone(), workspace.clone(), input.parts).await {
            Ok(prepared) => prepared,
            Err(error) => return Err(error),
        };
    let storage = workspace.storage.clone();
    let session_id = prepared.context.session_id.clone();

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

    let mut stream_state = StreamProcessor::new();
    let store_context = StorePartContext::new(
        &storage,
        bus,
        &request_id,
        &session_id,
        &assistant_message.info.id,
    );
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
    Ok(SessionCommandOutput {
        context: prepared.context,
        user_message: prepared.user_message,
        assistant_message: assistant_message.info,
        reply_parts,
    })
}

async fn prepare_session_message(
    sessions: Arc<SessionManager>,
    workspace: WorkspaceInstance,
    parts: Vec<SessionCommandInputPart>,
) -> CoreResult<PreparedSessionCommand> {
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

    let mut user_message = Message::from_parts(&context.session_id, "user", Vec::new());
    user_message.parts = map_input_parts(parts, &context.session_id, &user_message.info.id);
    Message::store(&storage, &user_message).await?;

    let message_history = Message::load(&storage, &context.session_id).await?;
    let prompt_messages = message_history
        .iter()
        .flat_map(Message::to_prompt_messages)
        .collect();

    Ok(PreparedSessionCommand {
        context,
        active_extension_ids,
        user_message: user_message.info,
        prompt_messages,
    })
}

fn map_input_parts(
    parts: Vec<SessionCommandInputPart>,
    session_id: &str,
    message_id: &str,
) -> Vec<MessagePart> {
    parts
        .into_iter()
        .map(|part| match part {
            SessionCommandInputPart::Text(part) => MessagePart::Text(TextPart {
                base: PartBase::new(session_id, message_id),
                text: part.text,
            }),
            SessionCommandInputPart::Extension(part) => MessagePart::Extension(ExtensionPart {
                base: PartBase::new(session_id, message_id),
                extension_id: part.extension_id,
                name: part.name,
                kind: part.kind,
                source: part.source,
            }),
            SessionCommandInputPart::File(part) => MessagePart::Text(TextPart {
                base: PartBase::new(session_id, message_id),
                text: map_file_input_to_text(part),
            }),
        })
        .collect()
}

fn map_file_input_to_text(part: SessionCommandFilePartInput) -> String {
    if !part.path.trim().is_empty() {
        return part.path;
    }

    if let Some(source) = part.source {
        if !source.value.trim().is_empty() {
            return source.value;
        }
    }

    if !part.name.trim().is_empty() {
        return format!("#{}", part.name);
    }

    "#file".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    use crate::bus::Bus;
    use crate::message::{MessagePart, ReasoningPart, TextPart, ToolState};
    use crate::session::SessionContext;
    use crate::session::SessionManager;
    use crate::workspace::WorkspaceInstance;
    use llm_kit_core::stream_text::TextStreamPart;
    use llm_kit_provider_utils::tool::{ToolCall, ToolError, ToolResult};

    fn text_input_part(text: &str) -> SessionCommandInputPart {
        SessionCommandInputPart::Text(SessionCommandTextPartInput {
            text: text.to_string(),
        })
    }

    fn file_input_part(path: &str, name: &str) -> SessionCommandInputPart {
        SessionCommandInputPart::File(SessionCommandFilePartInput {
            path: path.to_string(),
            name: name.to_string(),
            entry_type: Some("file".to_string()),
            source: Some(FilePartSourceText {
                value: format!("#{name}"),
                start: 0,
                end: name.len() as i64 + 1,
            }),
        })
    }

    #[tokio::test]
    async fn prepare_session_message_stores_user_message() {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::new(dir.path()).await.expect("workspace"));
        let sessions = Arc::new(SessionManager::new(workspace.clone()));

        let output = prepare_session_message(
            sessions,
            workspace.as_ref().clone(),
            vec![text_input_part("hello")],
        )
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
    async fn prepare_session_message_stores_extension_part() {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::new(dir.path()).await.expect("workspace"));
        let sessions = Arc::new(SessionManager::new(workspace.clone()));

        let output = prepare_session_message(
            sessions,
            workspace.as_ref().clone(),
            vec![
                SessionCommandInputPart::Extension(SessionCommandExtensionPartInput {
                    extension_id: "filesystem".to_string(),
                    name: "Filesystem".to_string(),
                    kind: Some("built-in".to_string()),
                    source: Some(FilePartSourceText {
                        value: "@filesystem".to_string(),
                        start: 0,
                        end: 11,
                    }),
                }),
                text_input_part("hello"),
            ],
        )
        .await
        .expect("command");

        let messages = Message::load(&workspace.storage, &output.context.session_id)
            .await
            .expect("load");
        let parts = &messages[0].parts;
        assert!(parts.iter().any(|part| matches!(
            part,
            MessagePart::Extension(part)
                if part.extension_id == "filesystem"
                    && part.name == "Filesystem"
                    && part.kind.as_deref() == Some("built-in")
                    && matches!(
                        part.source.as_ref(),
                        Some(source)
                            if source.value == "@filesystem" && source.start == 0 && source.end == 11
                    )
        )));
        assert!(parts.iter().any(|part| matches!(
            part,
            MessagePart::Text(TextPart { text, .. }) if text == "hello"
        )));
    }

    #[tokio::test]
    async fn prepare_session_message_maps_file_input_to_text_part_path() {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::new(dir.path()).await.expect("workspace"));
        let sessions = Arc::new(SessionManager::new(workspace.clone()));

        let output = prepare_session_message(
            sessions,
            workspace.as_ref().clone(),
            vec![
                file_input_part("/tmp/note.md", "note.md"),
                text_input_part("summarize"),
            ],
        )
        .await
        .expect("command");

        let messages = Message::load(&workspace.storage, &output.context.session_id)
            .await
            .expect("load");
        let parts = &messages[0].parts;

        assert!(parts.iter().any(|part| matches!(
            part,
            MessagePart::Text(TextPart { text, .. }) if text == "/tmp/note.md"
        )));
        assert!(parts.iter().any(|part| matches!(
            part,
            MessagePart::Text(TextPart { text, .. }) if text == "summarize"
        )));
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
        let result = run_session_command(
            sessions,
            workspace.as_ref().clone(),
            &llm,
            &bus,
            SessionCommandInput {
                request_id: "request-1".to_string(),
                parts: vec![text_input_part("hello")],
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

        let context = StorePartContext::new(
            &workspace.storage,
            &bus,
            &request_id,
            &session_id,
            &assistant.info.id,
        );
        let mut state = StreamProcessor::new();

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

        assert_eq!(state.mapped_parts().len(), 1);
        assert!(matches!(
            state.mapped_parts().first(),
            Some(MessagePart::Text(TextPart { text, .. })) if text == "hello"
        ));

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

        let context = StorePartContext::new(
            &workspace.storage,
            &bus,
            &request_id,
            &session_id,
            &assistant.info.id,
        );
        let mut state = StreamProcessor::new();

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

        let context = StorePartContext::new(
            &workspace.storage,
            &bus,
            &request_id,
            &session_id,
            &assistant.info.id,
        );
        let mut state = StreamProcessor::new();

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
    async fn stream_state_updates_tool_part_in_place_on_tool_result() {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::new(dir.path()).await.expect("workspace"));
        let bus = Bus::new(16);
        let request_id = "request_1".to_string();
        let session_id = "session_1".to_string();
        let assistant = Message::from_parts(&session_id, "assistant", Vec::new());
        Message::store_info(&workspace.storage, &assistant.info)
            .await
            .expect("store info");

        let context = StorePartContext::new(
            &workspace.storage,
            &bus,
            &request_id,
            &session_id,
            &assistant.info.id,
        );
        let mut state = StreamProcessor::new();

        state
            .on_part(
                TextStreamPart::ToolCall {
                    tool_call: ToolCall::new(
                        "call_1",
                        "test_tool",
                        serde_json::json!({"input": true}),
                    ),
                },
                &context,
            )
            .await
            .expect("tool call");

        let first_part_id = match state.mapped_parts().first() {
            Some(MessagePart::Tool(part)) => part.base.id.clone(),
            _ => panic!("expected tool part"),
        };

        state
            .on_part(
                TextStreamPart::ToolResult {
                    tool_result: ToolResult::new(
                        "call_1",
                        "test_tool",
                        serde_json::json!({"input": true}),
                        serde_json::json!({"ok": true}),
                    ),
                },
                &context,
            )
            .await
            .expect("tool result");

        assert_eq!(state.mapped_parts().len(), 1);
        assert!(matches!(
            state.mapped_parts().first(),
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

        let context = StorePartContext::new(
            &workspace.storage,
            &bus,
            &request_id,
            &session_id,
            &assistant.info.id,
        );
        let mut state = StreamProcessor::new();
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

        let context = StorePartContext::new(
            &workspace.storage,
            &bus,
            &request_id,
            &session_id,
            &assistant.info.id,
        );
        let mut state = StreamProcessor::new();
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

        let context = StorePartContext::new(
            &workspace.storage,
            &bus,
            &request_id,
            &session_id,
            &assistant.info.id,
        );
        let mut state = StreamProcessor::new();
        state
            .on_part(TextStreamPart::Start, &context)
            .await
            .expect("start");

        assert!(state.mapped_parts().is_empty());
    }
}
