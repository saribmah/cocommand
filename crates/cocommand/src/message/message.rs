use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};
use crate::storage::SharedStorage;

use crate::message::parts::MessagePart;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionMessage {
    pub seq: u64,
    pub timestamp: String,
    pub role: String,
    pub text: String,
}

const DEFAULT_CONTEXT_LIMIT: usize = 50;

impl Message {
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

pub fn messages_for_prompt(
    messages: Vec<MessageWithParts>,
    limit: Option<usize>,
) -> Vec<SessionMessage> {
    let cap = limit.unwrap_or(DEFAULT_CONTEXT_LIMIT);
    let messages = if messages.len() > cap {
        messages[messages.len() - cap..].to_vec()
    } else {
        messages
    };
    messages
        .into_iter()
        .enumerate()
        .map(|(index, item)| SessionMessage {
            seq: (index as u64).saturating_add(1),
            timestamp: item.info.created_at,
            role: item.info.role,
            text: render_message_text(&item.parts),
        })
        .collect()
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
