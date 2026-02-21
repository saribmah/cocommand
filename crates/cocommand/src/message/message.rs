use cocommand_llm::message::info::MessageInfo;
use cocommand_llm::message::parts::MessagePart;
use cocommand_llm::message::Message;

use crate::error::{CoreError, CoreResult};
use crate::storage::SharedStorage;
use crate::utils::time::now_rfc3339;

/// Storage operations for messages. The `Message` type itself lives in `cocommand_llm`.
pub struct MessageStorage;

impl MessageStorage {
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
