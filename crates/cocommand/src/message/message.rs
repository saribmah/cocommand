use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};
use crate::storage::SharedStorage;
use crate::utils::time::now_rfc3339;

use crate::message::parts::{MessagePart, TextPart};
use llm_kit_provider_utils::message::{AssistantMessage, Message as LlmMessage, UserMessage};

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

    pub fn to_prompt(role: &str, parts: &[MessagePart]) -> Option<LlmMessage> {
        let text = render_message_text(parts);
        match role {
            "user" => Some(LlmMessage::User(UserMessage::new(text))),
            "assistant" => Some(LlmMessage::Assistant(AssistantMessage::new(text))),
            _ => None,
        }
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
}

pub fn render_message_text(parts: &[MessagePart]) -> String {
    parts
        .iter()
        .filter_map(|part| match part {
            MessagePart::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

async fn load_message_info(
    storage: &SharedStorage,
    session_id: &str,
    message_id: &str,
) -> CoreResult<Option<MessageInfo>> {
    let value = storage
        .read(&["messages", session_id, message_id])
        .await?;
    match value {
        Some(value) => serde_json::from_value(value).map(Some).map_err(|error| {
            CoreError::Internal(format!("failed to parse message info: {error}"))
        }),
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
