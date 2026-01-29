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

    pub fn session(&self) -> CoreResult<Session> {
        self.ensure_session()
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
        let mut session = manager.session().expect("session");
        session.record_message("hello").expect("record");
        let ctx = session.context(None).expect("context");
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
        let first = manager.session().expect("session").context(None).expect("context");
        let second = manager.session().expect("session").context(None).expect("context");
        assert_ne!(first.session_id, second.session_id);
    }
}
