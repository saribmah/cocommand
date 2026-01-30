use std::sync::{Arc, Mutex};

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

    pub fn with_session_mut<F, R>(&self, handler: F) -> CoreResult<R>
    where
        F: FnOnce(&mut Session) -> CoreResult<R>,
    {
        let mut guard = self
            .active
            .lock()
            .map_err(|_| CoreError::Internal("session manager lock poisoned".to_string()))?;
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
        handler(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn manager_records_messages() {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::new(dir.path()).expect("workspace"));
        let manager = SessionManager::new(workspace);
        let ctx = manager
            .with_session_mut(|session| {
                session.record_message("hello")?;
                session.context(None)
            })
            .expect("record");
        assert_eq!(ctx.messages.len(), 1);
        assert_eq!(ctx.messages[0].text, "hello");
    }

    #[test]
    fn manager_rollover_resets_cache() {
        let dir = tempdir().expect("tempdir");
        let mut workspace = WorkspaceInstance::new(dir.path()).expect("workspace");
        workspace.config.preferences.session.duration_seconds = 0;
        let workspace = Arc::new(workspace);
        let manager = SessionManager::new(workspace.clone());
        let first = manager
            .with_session_mut(|session| session.context(None))
            .expect("context");
        let second = manager
            .with_session_mut(|session| session.context(None))
            .expect("context");
        assert_ne!(first.session_id, second.session_id);
    }
}
