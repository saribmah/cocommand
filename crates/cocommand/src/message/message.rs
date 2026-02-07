use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};
use crate::storage::SharedStorage;
use crate::utils::time::now_rfc3339;

use crate::message::parts::{MessagePart, TextPart};
use llm_kit_core::stream_text::StreamTextResult;
use llm_kit_provider_utils::message::{
    AssistantContentPart, AssistantMessage, DataContent, FilePart as LlmFilePart,
    Message as LlmMessage, ReasoningPart as LlmReasoningPart, TextPart as LlmTextPart,
    ToolCallPart as LlmToolCallPart, ToolContentPart, ToolMessage, ToolResultOutput,
    ToolResultPart as LlmToolResultPart, UserContentPart, UserMessage,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub id: String,
    pub role: MessageRole,
    pub timestamp: String,
    pub parts: Vec<MessagePart>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageInfo {
    pub id: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub role: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

pub type UserMessageInfo = MessageInfo;
pub type AssistantMessageInfo = MessageInfo;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageWithParts {
    pub info: MessageInfo,
    pub parts: Vec<MessagePart>,
}

impl Message {
    pub fn from_text(session_id: &str, role: &str, text: &str) -> MessageWithParts {
        let timestamp = now_rfc3339();
        MessageWithParts {
            info: MessageInfo {
                id: Uuid::now_v7().to_string(),
                session_id: session_id.to_string(),
                role: role.to_string(),
                created_at: timestamp.clone(),
                updated_at: timestamp,
            },
            parts: vec![MessagePart::Text(TextPart {
                text: text.to_string(),
            })],
        }
    }

    pub fn to_prompt_messages(message: &MessageWithParts) -> Vec<LlmMessage> {
        match message.info.role.as_str() {
            "user" => user_message_to_prompt(message),
            "assistant" => assistant_message_to_prompt(message),
            "tool" => tool_message_to_prompt(message),
            _ => Vec::new(),
        }
    }

    pub fn to_text(message: &MessageWithParts) -> String {
        message
            .parts
            .iter()
            .filter_map(|part| match part {
                MessagePart::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    pub async fn load(
        storage: &SharedStorage,
        session_id: &str,
    ) -> CoreResult<Vec<MessageWithParts>> {
        let mut message_ids = storage.list(&["messages", session_id]).await?;
        message_ids.sort();
        let mut items = Vec::new();
        for message_id in message_ids {
            if let Some(info) = load_message_info(storage, session_id, &message_id).await? {
                let parts = load_message_parts(storage, &message_id).await?;
                items.push(MessageWithParts { info, parts });
            }
        }
        Ok(items)
    }

    pub async fn store(storage: &SharedStorage, message: &MessageWithParts) -> CoreResult<()> {
        save_message_info(storage, &message.info).await?;
        for part in &message.parts {
            let part_id = Uuid::now_v7().to_string();
            save_message_part(storage, &message.info.id, &part_id, part).await?;
        }
        Ok(())
    }

    pub async fn store_info(storage: &SharedStorage, info: &MessageInfo) -> CoreResult<()> {
        save_message_info(storage, info).await
    }

    pub async fn store_part(
        storage: &SharedStorage,
        message_id: &str,
        part_id: &str,
        part: &MessagePart,
    ) -> CoreResult<()> {
        save_message_part(storage, message_id, part_id, part).await
    }

    pub async fn touch_info(storage: &SharedStorage, info: &mut MessageInfo) -> CoreResult<()> {
        info.updated_at = now_rfc3339();
        save_message_info(storage, info).await
    }

    pub async fn from_stream(
        session_id: &str,
        role: &str,
        result: &StreamTextResult,
    ) -> CoreResult<MessageWithParts> {
        let parts = crate::message::stream_result_to_parts(result).await?;
        let timestamp = now_rfc3339();
        Ok(MessageWithParts {
            info: MessageInfo {
                id: Uuid::now_v7().to_string(),
                session_id: session_id.to_string(),
                role: role.to_string(),
                created_at: timestamp.clone(),
                updated_at: timestamp,
            },
            parts,
        })
    }

    pub fn from_parts(session_id: &str, role: &str, parts: Vec<MessagePart>) -> MessageWithParts {
        let timestamp = now_rfc3339();
        MessageWithParts {
            info: MessageInfo {
                id: Uuid::now_v7().to_string(),
                session_id: session_id.to_string(),
                role: role.to_string(),
                created_at: timestamp.clone(),
                updated_at: timestamp,
            },
            parts,
        }
    }
}

fn user_message_to_prompt(message: &MessageWithParts) -> Vec<LlmMessage> {
    let parts: Vec<UserContentPart> = message
        .parts
        .iter()
        .filter_map(|part| match part {
            MessagePart::Text(text) => Some(UserContentPart::Text(LlmTextPart::new(&text.text))),
            MessagePart::File(file) => Some(UserContentPart::File(map_file_part(file))),
            MessagePart::Source(source) => Some(UserContentPart::Text(LlmTextPart::new(
                format_source(source),
            ))),
            _ => None,
        })
        .collect();
    if parts.is_empty() {
        return Vec::new();
    }
    vec![LlmMessage::User(UserMessage::with_parts(parts))]
}

fn assistant_message_to_prompt(message: &MessageWithParts) -> Vec<LlmMessage> {
    let assistant_parts: Vec<AssistantContentPart> = message
        .parts
        .iter()
        .filter_map(|part| match part {
            MessagePart::Text(text) => {
                Some(AssistantContentPart::Text(LlmTextPart::new(&text.text)))
            }
            MessagePart::Reasoning(reasoning) => Some(AssistantContentPart::Reasoning(
                LlmReasoningPart::new(&reasoning.text),
            )),
            MessagePart::ToolCall(call) => {
                Some(AssistantContentPart::ToolCall(LlmToolCallPart::new(
                    call.call_id.clone(),
                    call.tool_name.clone(),
                    call.input.clone(),
                )))
            }
            MessagePart::File(file) => Some(AssistantContentPart::File(map_file_part(file))),
            MessagePart::Source(source) => Some(AssistantContentPart::Text(LlmTextPart::new(
                format_source(source),
            ))),
            MessagePart::ToolResult(_) => None,
            MessagePart::ToolError(_) => None,
        })
        .collect();
    let tool_parts: Vec<ToolContentPart> = message
        .parts
        .iter()
        .filter_map(|part| match part {
            MessagePart::ToolResult(result) => {
                let output = map_tool_result_output(result);
                Some(ToolContentPart::ToolResult(LlmToolResultPart::new(
                    result.call_id.clone(),
                    result.tool_name.clone(),
                    output,
                )))
            }
            MessagePart::ToolError(error) => {
                let output = map_tool_error_output(error);
                Some(ToolContentPart::ToolResult(LlmToolResultPart::new(
                    error.call_id.clone(),
                    error.tool_name.clone(),
                    output,
                )))
            }
            _ => None,
        })
        .collect();
    let mut messages = Vec::new();
    if !assistant_parts.is_empty() {
        messages.push(LlmMessage::Assistant(AssistantMessage::with_parts(
            assistant_parts,
        )));
    }
    if !tool_parts.is_empty() {
        messages.push(LlmMessage::Tool(ToolMessage::new(tool_parts)));
    }
    messages
}

fn tool_message_to_prompt(message: &MessageWithParts) -> Vec<LlmMessage> {
    let parts: Vec<ToolContentPart> = message
        .parts
        .iter()
        .filter_map(|part| match part {
            MessagePart::ToolResult(result) => {
                let output = map_tool_result_output(result);
                Some(ToolContentPart::ToolResult(LlmToolResultPart::new(
                    result.call_id.clone(),
                    result.tool_name.clone(),
                    output,
                )))
            }
            MessagePart::ToolError(error) => {
                let output = map_tool_error_output(error);
                Some(ToolContentPart::ToolResult(LlmToolResultPart::new(
                    error.call_id.clone(),
                    error.tool_name.clone(),
                    output,
                )))
            }
            _ => None,
        })
        .collect();
    if parts.is_empty() {
        return Vec::new();
    }
    vec![LlmMessage::Tool(ToolMessage::new(parts))]
}

fn map_tool_result_output(result: &crate::message::parts::ToolResultPart) -> ToolResultOutput {
    if result.is_error {
        if let Some(text) = result.output.as_str() {
            ToolResultOutput::error_text(text)
        } else {
            ToolResultOutput::error_json(result.output.clone())
        }
    } else if let Some(text) = result.output.as_str() {
        ToolResultOutput::text(text)
    } else {
        ToolResultOutput::json(result.output.clone())
    }
}

fn map_tool_error_output(error: &crate::message::parts::ToolErrorPart) -> ToolResultOutput {
    if let Some(text) = error.error.as_str() {
        ToolResultOutput::error_text(text)
    } else {
        ToolResultOutput::error_json(error.error.clone())
    }
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

fn format_source(source: &crate::message::parts::SourcePart) -> String {
    match (&source.title, &source.url, &source.filename) {
        (Some(title), Some(url), _) => format!("Source: {} ({})", title, url),
        (Some(title), None, Some(filename)) => format!("Source: {} ({})", title, filename),
        (Some(title), None, None) => format!("Source: {}", title),
        (None, Some(url), _) => format!("Source: {}", url),
        (None, None, Some(filename)) => format!("Source: {}", filename),
        (None, None, None) => format!("Source: {}", source.id),
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
