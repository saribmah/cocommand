use std::sync::{Arc, Mutex};

use crate::error::{CoreError, CoreResult};
use crate::session::session::Session;
use crate::session::SessionContext;
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

    pub fn record_message(&self, text: &str) -> CoreResult<SessionContext> {
        let mut session = self.ensure_session()?;
        session.record_message(text)?;
        session.context(None)
    }

    pub fn context(
        &self,
        session_id: Option<&str>,
        limit: Option<usize>,
    ) -> CoreResult<SessionContext> {
        let session = self.ensure_session()?;
        session.context_with_id(session_id, limit)
    }

    pub fn open_application(&self, app_id: &str) -> CoreResult<SessionContext> {
        let mut session = self.ensure_session()?;
        session.open_application(app_id);
        session.context(None)
    }

    pub fn close_application(&self, app_id: &str) -> CoreResult<SessionContext> {
        let mut session = self.ensure_session()?;
        session.close_application(app_id);
        session.context(None)
    }

    fn ensure_session(&self) -> CoreResult<Session> {
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

        guard
            .as_ref()
            .cloned()
            .ok_or_else(|| CoreError::Internal("failed to initialize session".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn manager_records_messages() {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::load(dir.path()).expect("workspace"));
        let manager = SessionManager::new(workspace);
        let ctx = manager.record_message("hello").expect("record");
        assert_eq!(ctx.messages.len(), 1);
        assert_eq!(ctx.messages[0].text, "hello");
    }

    #[test]
    fn manager_rollover_resets_cache() {
        let dir = tempdir().expect("tempdir");
        let mut workspace = WorkspaceInstance::load(dir.path()).expect("workspace");
        workspace.config.preferences.session.duration_seconds = 0;
        let workspace = Arc::new(workspace);
        let manager = SessionManager::new(workspace.clone());
        let first = manager.context(None, None).expect("context");
        let second = manager.context(None, None).expect("context");
        assert_ne!(first.session_id, second.session_id);
    }
}
