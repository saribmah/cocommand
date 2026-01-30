use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};
use crate::message::{MessageInfo, MessagePart, MessageWithParts, TextPart};
use crate::utils::time::{now_rfc3339, now_secs};
use crate::session::application_cache::ApplicationCache;
use crate::workspace::WorkspaceInstance;

const DEFAULT_CONTEXT_LIMIT: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub seq: u64,
    pub timestamp: String,
    pub role: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub workspace_id: String,
    pub session_id: String,
    pub started_at: u64,
    pub ended_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub workspace_id: String,
    pub started_at: u64,
    pub ended_at: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct Session {
    workspace: Arc<WorkspaceInstance>,
    pub(crate) session_id: String,
    pub(crate) started_at: u64,
    ended_at: Option<u64>,
    application_cache: ApplicationCache,
}

impl Session {
    pub async fn new(workspace: Arc<WorkspaceInstance>) -> CoreResult<Self> {
        let ttl = workspace.config.preferences.session.duration_seconds;
        let max_apps = workspace.config.preferences.application_cache.max_applications;
        let cache = ApplicationCache::new(max_apps, ttl);
        let session = Self {
            workspace,
            session_id: Uuid::now_v7().to_string(),
            started_at: now_secs(),
            ended_at: None,
            application_cache: cache,
        };
        session.persist_info().await?;
        Ok(session)
    }

    pub fn from_info(workspace: Arc<WorkspaceInstance>, info: SessionInfo) -> CoreResult<Self> {
        let ttl = workspace.config.preferences.session.duration_seconds;
        let max_apps = workspace.config.preferences.application_cache.max_applications;
        let cache = ApplicationCache::new(max_apps, ttl);
        Ok(Self {
            workspace,
            session_id: info.id,
            started_at: info.started_at,
            ended_at: info.ended_at,
            application_cache: cache,
        })
    }

    pub async fn record_message(&mut self, text: &str) -> CoreResult<()> {
        let part = MessagePart::Text(TextPart {
            text: text.to_string(),
        });
        self.record_message_with_role("user", vec![part]).await
    }

    pub async fn record_assistant_message(&mut self, text: &str) -> CoreResult<()> {
        let part = MessagePart::Text(TextPart {
            text: text.to_string(),
        });
        self.record_message_with_role("assistant", vec![part]).await
    }

    pub async fn record_assistant_parts(&mut self, parts: Vec<MessagePart>) -> CoreResult<()> {
        self.record_message_with_role("assistant", parts).await
    }

    async fn record_message_with_role(
        &mut self,
        role: &str,
        parts: Vec<MessagePart>,
    ) -> CoreResult<()> {
        let message_id = Uuid::now_v7().to_string();
        let timestamp = now_rfc3339();
        let info = MessageInfo {
            id: message_id.clone(),
            session_id: self.session_id.clone(),
            role: role.to_string(),
            created_at: timestamp.clone(),
            updated_at: timestamp,
        };
        self.save_message_info(&info).await?;
        for part in parts {
            let part_id = Uuid::now_v7().to_string();
            self.save_message_part(&message_id, &part_id, &part)
                .await?;
        }
        Ok(())
    }

    pub async fn context(&self, limit: Option<usize>) -> CoreResult<SessionContext> {
        self.context_with_id(Some(&self.session_id), limit).await
    }

    pub async fn context_with_id(
        &self,
        session_id: Option<&str>,
        _limit: Option<usize>,
    ) -> CoreResult<SessionContext> {
        if let Some(id) = session_id {
            if id != self.session_id {
                return Err(CoreError::InvalidInput("session not found".to_string()));
            }
        }
        Ok(SessionContext {
            workspace_id: self.workspace.config.workspace_id.clone(),
            session_id: self.session_id.clone(),
            started_at: self.started_at,
            ended_at: self.ended_at,
        })
    }

    pub async fn messages(&self) -> CoreResult<Vec<MessageWithParts>> {
        self.load_messages_with_parts().await
    }

    pub async fn messages_for_prompt(&self, limit: Option<usize>) -> CoreResult<Vec<SessionMessage>> {
        let cap = limit.unwrap_or(DEFAULT_CONTEXT_LIMIT);
        let messages_with_parts = self.messages().await?;
        let messages_with_parts = if messages_with_parts.len() > cap {
            messages_with_parts[messages_with_parts.len() - cap..].to_vec()
        } else {
            messages_with_parts
        };
        Ok(messages_with_parts
            .into_iter()
            .enumerate()
            .map(|(index, item)| SessionMessage {
                seq: (index as u64).saturating_add(1),
                timestamp: item.info.created_at,
                role: item.info.role,
                text: render_message_text(&item.parts),
            })
            .collect())
    }

    pub fn open_application(&mut self, app_id: &str) {
        self.application_cache
            .open_application(app_id, now_secs());
    }

    pub fn close_application(&mut self, app_id: &str) {
        self.application_cache.close_application(app_id);
    }

    pub async fn destroy(&mut self) -> CoreResult<()> {
        self.ended_at = Some(now_secs());
        self.application_cache = ApplicationCache::new(0, 1);
        self.persist_info().await?;
        Ok(())
    }

    async fn load_messages_with_parts(&self) -> CoreResult<Vec<MessageWithParts>> {
        let storage = self.workspace.storage.clone();
        let mut message_ids = storage
            .list(&["messages", &self.session_id])
            .await?;
        message_ids.sort();
        let mut items = Vec::new();
        for message_id in message_ids {
            if let Some(info) = self.load_message_info(&message_id).await? {
                let parts = self.load_message_parts(&message_id).await?;
                items.push(MessageWithParts { info, parts });
            }
        }
        Ok(items)
    }

    async fn load_message_info(&self, message_id: &str) -> CoreResult<Option<MessageInfo>> {
        let value = self
            .workspace
            .storage
            .read(&["messages", &self.session_id, message_id])
            .await?;
        match value {
            Some(value) => serde_json::from_value(value).map(Some).map_err(|error| {
                CoreError::Internal(format!("failed to parse message info: {error}"))
            }),
            None => Ok(None),
        }
    }

    async fn save_message_info(&self, info: &MessageInfo) -> CoreResult<()> {
        let value = serde_json::to_value(info).map_err(|error| {
            CoreError::Internal(format!("failed to serialize message info: {error}"))
        })?;
        self.workspace
            .storage
            .write(&["messages", &self.session_id, &info.id], &value)
            .await
    }

    async fn load_message_parts(&self, message_id: &str) -> CoreResult<Vec<MessagePart>> {
        let storage = self.workspace.storage.clone();
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
        &self,
        message_id: &str,
        part_id: &str,
        part: &MessagePart,
    ) -> CoreResult<()> {
        let value = serde_json::to_value(part).map_err(|error| {
            CoreError::Internal(format!("failed to serialize message part: {error}"))
        })?;
        self.workspace
            .storage
            .write(&["part", message_id, part_id], &value)
            .await
    }

    async fn persist_info(&self) -> CoreResult<()> {
        let info = SessionInfo {
            id: self.session_id.clone(),
            workspace_id: self.workspace.config.workspace_id.clone(),
            started_at: self.started_at,
            ended_at: self.ended_at,
        };
        let value = serde_json::to_value(info).map_err(|error| {
            CoreError::Internal(format!("failed to serialize session info: {error}"))
        })?;
        let workspace_id = self.workspace.config.workspace_id.clone();
        self.workspace
            .storage
            .write(&["session", &workspace_id, &self.session_id], &value)
            .await
    }
}

pub(crate) fn render_message_text(parts: &[MessagePart]) -> String {
    parts
        .iter()
        .filter_map(|part| match part {
            MessagePart::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn session_records_messages() {
        let dir = tempdir().expect("tempdir");
        let workspace = WorkspaceInstance::new(dir.path()).await.expect("workspace");
        let workspace = Arc::new(workspace);
        let mut session = Session::new(workspace).await.expect("session");
        session.record_message("hello").await.expect("record");
        let messages = session.messages_for_prompt(None).await.expect("messages");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "hello");
    }
}
