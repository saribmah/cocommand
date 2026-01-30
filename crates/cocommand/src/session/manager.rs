use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::error::{CoreError, CoreResult};
use crate::session::session::{Session, SessionInfo};
use crate::utils::time::now_secs;
use crate::workspace::WorkspaceInstance;

pub struct SessionManager {
    workspace: Arc<WorkspaceInstance>,
    active: Mutex<Option<Session>>,
}

impl SessionManager {
    pub fn new(workspace: Arc<WorkspaceInstance>) -> Self {
        Self {
            workspace,
            active: Mutex::new(None),
        }
    }

    pub async fn with_session_mut<F, R>(&self, handler: F) -> CoreResult<R>
    where
        for<'a> F: FnOnce(&'a mut Session) -> Pin<Box<dyn Future<Output = CoreResult<R>> + Send + 'a>>,
    {
        let mut guard = self
            .active
            .lock()
            .await;
        let now = now_secs();
        let duration = self.workspace.config.preferences.session.duration_seconds;

        let needs_new = match guard.as_ref() {
            Some(existing) => now.saturating_sub(existing.started_at) >= duration,
            None => true,
        };

        if needs_new {
            if let Some(mut existing) = guard.take() {
                existing.destroy().await?;
            }
            let session = if let Some(resumed) = self.load_latest_session(now, duration).await? {
                resumed
            } else {
                Session::new(self.workspace.clone()).await?
            };
            *guard = Some(session);
        }
        let session = guard
            .as_mut()
            .ok_or_else(|| CoreError::Internal("failed to initialize session".to_string()))?;
        handler(session).await
    }

    async fn load_latest_session(
        &self,
        now: u64,
        duration: u64,
    ) -> CoreResult<Option<Session>> {
        let storage = self.workspace.storage.clone();
        let workspace_id = self.workspace.config.workspace_id.clone();
        let mut session_ids = storage.list(&["session", &workspace_id]).await?;
        if session_ids.is_empty() {
            return Ok(None);
        }
        session_ids.sort();
        let last_id = match session_ids.last() {
            Some(id) => id.clone(),
            None => return Ok(None),
        };
        let value = storage.read(&["session", &workspace_id, &last_id]).await?;
        let Some(value) = value else {
            return Ok(None);
        };
        let mut info: SessionInfo = serde_json::from_value(value).map_err(|error| {
            CoreError::Internal(format!("failed to parse session info: {error}"))
        })?;
        if info.ended_at.is_some() {
            return Ok(None);
        }
        if now.saturating_sub(info.started_at) >= duration {
            info.ended_at = Some(now);
            let serialized = serde_json::to_value(&info).map_err(|error| {
                CoreError::Internal(format!("failed to serialize session info: {error}"))
            })?;
            storage
                .write(&["session", &workspace_id, &info.id], &serialized)
                .await?;
            return Ok(None);
        }
        Ok(Some(Session::from_info(self.workspace.clone(), info)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{messages_for_prompt, Message, MessageInfo, MessagePart, MessageWithParts, TextPart};
    use crate::utils::time::now_rfc3339;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[tokio::test]
    async fn manager_records_messages() {
        let dir = tempdir().expect("tempdir");
        let workspace = WorkspaceInstance::new(dir.path()).await.expect("workspace");
        let storage = workspace.storage.clone();
        let workspace = Arc::new(workspace);
        let manager = SessionManager::new(workspace);
        let messages = manager
            .with_session_mut(|session| {
                let storage = storage.clone();
                Box::pin(async move {
                    let timestamp = now_rfc3339();
                    let message = MessageWithParts {
                        info: MessageInfo {
                            id: Uuid::now_v7().to_string(),
                            session_id: session.session_id.clone(),
                            role: "user".to_string(),
                            created_at: timestamp.clone(),
                            updated_at: timestamp,
                        },
                        parts: vec![MessagePart::Text(TextPart {
                            text: "hello".to_string(),
                        })],
                    };
                    Message::store(&storage, &message).await?;
                    let history = Message::load(&storage, &session.session_id).await?;
                    Ok(messages_for_prompt(history, None))
                })
            })
            .await
            .expect("record");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "hello");
    }

    #[tokio::test]
    async fn manager_rollover_resets_cache() {
        let dir = tempdir().expect("tempdir");
        let mut workspace = WorkspaceInstance::new(dir.path()).await.expect("workspace");
        workspace.config.preferences.session.duration_seconds = 0;
        let workspace = Arc::new(workspace);
        let manager = SessionManager::new(workspace.clone());
        let first = manager
            .with_session_mut(|session| Box::pin(async move { session.context(None).await }))
            .await
            .expect("context");
        let second = manager
            .with_session_mut(|session| Box::pin(async move { session.context(None).await }))
            .await
            .expect("context");
        assert_ne!(first.session_id, second.session_id);
    }
}
