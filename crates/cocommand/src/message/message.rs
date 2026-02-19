use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};
use crate::storage::SharedStorage;
use crate::utils::time::now_rfc3339;

use crate::message::info::MessageInfo;
use crate::message::parts::{MessagePart, PartBase, TextPart, ToolPart, ToolState};
use llm_kit_provider_utils::message::{
    AssistantContentPart, AssistantMessage, DataContent, FilePart as LlmFilePart,
    Message as LlmMessage, ReasoningPart as LlmReasoningPart, TextPart as LlmTextPart,
    ToolCallPart as LlmToolCallPart, ToolContentPart, ToolMessage, ToolResultOutput,
    ToolResultPart as LlmToolResultPart, UserContentPart, UserMessage,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct Message {
    pub info: MessageInfo,
    pub parts: Vec<MessagePart>,
}

impl Message {
    pub fn from_text(session_id: &str, role: &str, text: &str) -> Message {
        let timestamp = now_rfc3339();
        let info = MessageInfo {
            id: Uuid::now_v7().to_string(),
            session_id: session_id.to_string(),
            role: role.to_string(),
            created_at: timestamp.clone(),
            completed_at: None,
        };
        Message {
            info: info.clone(),
            parts: vec![MessagePart::Text(TextPart {
                base: PartBase::new(session_id, info.id.as_str()),
                text: text.to_string(),
            })],
        }
    }

    pub fn from_parts(session_id: &str, role: &str, parts: Vec<MessagePart>) -> Message {
        let timestamp = now_rfc3339();
        let info = MessageInfo {
            id: Uuid::now_v7().to_string(),
            session_id: session_id.to_string(),
            role: role.to_string(),
            created_at: timestamp.clone(),
            completed_at: None,
        };
        let parts = parts
            .into_iter()
            .map(|part| part.with_base(PartBase::new(session_id, info.id.as_str())))
            .collect();
        Message { info, parts }
    }

    pub fn to_prompt_messages(message: &Message) -> Vec<LlmMessage> {
        match message.info.role.as_str() {
            "user" => user_message_to_prompt(message),
            "assistant" => assistant_message_to_prompt(message),
            "tool" => tool_message_to_prompt(message),
            _ => Vec::new(),
        }
    }

    pub async fn load(storage: &SharedStorage, session_id: &str) -> CoreResult<Vec<Message>> {
        let mut message_ids = storage.list(&["messages", session_id]).await?;
        message_ids.sort();
        let mut items = Vec::new();
        for message_id in message_ids {
            if let Some(info) = load_message_info(storage, session_id, &message_id).await? {
                let parts = load_message_parts(storage, &message_id).await?;
                items.push(Message { info, parts });
            }
        }
        Ok(items)
    }

    pub async fn store(storage: &SharedStorage, message: &Message) -> CoreResult<()> {
        save_message_info(storage, &message.info).await?;
        for part in &message.parts {
            save_message_part(storage, &message.info.id, &part.base().id, part).await?;
        }
        Ok(())
    }

    pub async fn store_info(storage: &SharedStorage, info: &MessageInfo) -> CoreResult<()> {
        save_message_info(storage, info).await
    }

    pub async fn store_part(storage: &SharedStorage, part: &MessagePart) -> CoreResult<()> {
        let base = part.base();
        save_message_part(storage, &base.message_id, &base.id, part).await
    }

    pub async fn touch_info(storage: &SharedStorage, info: &mut MessageInfo) -> CoreResult<()> {
        info.completed_at = Some(now_rfc3339());
        save_message_info(storage, info).await
    }
}

fn user_message_to_prompt(message: &Message) -> Vec<LlmMessage> {
    let parts: Vec<UserContentPart> = message
        .parts
        .iter()
        .filter_map(|part| match part {
            MessagePart::Text(text) => Some(UserContentPart::Text(LlmTextPart::new(&text.text))),
            MessagePart::File(file) => Some(UserContentPart::File(map_file_part(file))),
            _ => None,
        })
        .collect();
    if parts.is_empty() {
        return Vec::new();
    }
    vec![LlmMessage::User(UserMessage::with_parts(parts))]
}

fn assistant_message_to_prompt(message: &Message) -> Vec<LlmMessage> {
    let mut assistant_parts = Vec::new();
    for part in &message.parts {
        match part {
            MessagePart::Text(text) => {
                assistant_parts.push(AssistantContentPart::Text(LlmTextPart::new(&text.text)));
            }
            MessagePart::Reasoning(reasoning) => {
                assistant_parts.push(AssistantContentPart::Reasoning(LlmReasoningPart::new(
                    &reasoning.text,
                )));
            }
            MessagePart::Tool(tool) => {
                if let Some(assistant_part) = map_tool_to_assistant_content(tool) {
                    assistant_parts.push(assistant_part);
                }
            }
            MessagePart::Extension(_) => {}
            MessagePart::File(file) => {
                assistant_parts.push(AssistantContentPart::File(map_file_part(file)));
            }
        }
    }
    let mut messages = Vec::new();
    if !assistant_parts.is_empty() {
        messages.push(LlmMessage::Assistant(AssistantMessage::with_parts(
            assistant_parts,
        )));
    }
    messages
}

fn tool_message_to_prompt(message: &Message) -> Vec<LlmMessage> {
    let parts: Vec<ToolContentPart> = message
        .parts
        .iter()
        .filter_map(|part| match part {
            MessagePart::Tool(tool) => map_tool_to_tool_content(tool),
            _ => None,
        })
        .collect();
    if parts.is_empty() {
        return Vec::new();
    }
    vec![LlmMessage::Tool(ToolMessage::new(parts))]
}

fn map_tool_to_assistant_content(tool: &ToolPart) -> Option<AssistantContentPart> {
    match &tool.state {
        ToolState::Pending(state) => Some(AssistantContentPart::ToolCall(LlmToolCallPart::new(
            tool.call_id.clone(),
            tool.tool.clone(),
            Value::Object(state.input.clone()),
        ))),
        ToolState::Running(state) => Some(AssistantContentPart::ToolCall(LlmToolCallPart::new(
            tool.call_id.clone(),
            tool.tool.clone(),
            Value::Object(state.input.clone()),
        ))),
        ToolState::Completed(_) => None,
        ToolState::Error(_) => None,
    }
}

fn map_tool_to_tool_content(tool: &ToolPart) -> Option<ToolContentPart> {
    let output = match &tool.state {
        ToolState::Completed(state) => ToolResultOutput::text(&state.output),
        ToolState::Error(state) => ToolResultOutput::error_text(&state.error),
        ToolState::Pending(_) | ToolState::Running(_) => return None,
    };
    Some(ToolContentPart::ToolResult(LlmToolResultPart::new(
        tool.call_id.clone(),
        tool.tool.clone(),
        output,
    )))
}

fn map_file_part(file: &crate::message::parts::FilePart) -> LlmFilePart {
    let part = LlmFilePart::from_data(
        DataContent::base64(file.base64.clone()),
        file.media_type.clone(),
    );
    match &file.name {
        Some(name) => part.with_filename(name.clone()),
        None => part,
    }
}

async fn load_message_info(
    storage: &SharedStorage,
    session_id: &str,
    message_id: &str,
) -> CoreResult<Option<MessageInfo>> {
    let value = storage.read(&["messages", session_id, message_id]).await?;
    match value {
        Some(value) => serde_json::from_value(value)
            .map(Some)
            .map_err(|error| CoreError::Internal(format!("failed to parse message info: {error}"))),
        None => Ok(None),
    }
}

async fn save_message_info(storage: &SharedStorage, info: &MessageInfo) -> CoreResult<()> {
    let value = serde_json::to_value(info).map_err(|error| {
        CoreError::Internal(format!("failed to serialize message info: {error}"))
    })?;
    storage
        .write(&["messages", &info.session_id, &info.id], &value)
        .await
}

async fn load_message_parts(
    storage: &SharedStorage,
    message_id: &str,
) -> CoreResult<Vec<MessagePart>> {
    let mut part_ids = storage.list(&["part", message_id]).await?;
    part_ids.sort();
    let mut parts = Vec::new();
    for part_id in part_ids {
        if let Some(value) = storage.read(&["part", message_id, &part_id]).await? {
            let part: MessagePart = serde_json::from_value(value).map_err(|error| {
                CoreError::Internal(format!("failed to parse message part: {error}"))
            })?;
            parts.push(part);
        }
    }
    Ok(parts)
}

async fn save_message_part(
    storage: &SharedStorage,
    message_id: &str,
    part_id: &str,
    part: &MessagePart,
) -> CoreResult<()> {
    let value = serde_json::to_value(part).map_err(|error| {
        CoreError::Internal(format!("failed to serialize message part: {error}"))
    })?;
    storage.write(&["part", message_id, part_id], &value).await
}
