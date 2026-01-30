use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::error::{CoreError, CoreResult};
use crate::session::session::Session;
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
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let duration = self.workspace.config.preferences.session.duration_seconds;

        let needs_new = match guard.as_ref() {
            Some(existing) => now.saturating_sub(existing.started_at) >= duration,
            None => true,
        };

        if needs_new {
            if let Some(mut existing) = guard.take() {
                existing.destroy()?;
            }
            let session = Session::new(self.workspace.clone())?;
            *guard = Some(session);
        }
        let session = guard
            .as_mut()
            .ok_or_else(|| CoreError::Internal("failed to initialize session".to_string()))?;
        handler(session).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn manager_records_messages() {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::new(dir.path()).expect("workspace"));
        let manager = SessionManager::new(workspace);
        let messages = manager
            .with_session_mut(|session| {
                Box::pin(async move {
                    session.record_message("hello").await?;
                    session.messages_for_prompt(None).await
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
        let mut workspace = WorkspaceInstance::new(dir.path()).expect("workspace");
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
