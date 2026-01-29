use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};
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
    pub messages: Vec<SessionMessage>,
}

#[derive(Debug, Clone)]
pub struct Session {
    workspace: Arc<WorkspaceInstance>,
    pub(crate) session_id: String,
    pub(crate) started_at: u64,
    ended_at: Option<u64>,
    messages: Vec<SessionMessage>,
    next_seq: u64,
    application_cache: ApplicationCache,
}

impl Session {
    pub fn new(workspace: Arc<WorkspaceInstance>) -> CoreResult<Self> {
        let ttl = workspace.config.preferences.session.duration_seconds;
        let max_apps = workspace.config.preferences.application_cache.max_applications;
        let cache = ApplicationCache::new(max_apps, ttl);
        Ok(Self {
            workspace,
            session_id: Uuid::new_v4().to_string(),
            started_at: now_secs(),
            ended_at: None,
            messages: Vec::new(),
            next_seq: 1,
            application_cache: cache,
        })
    }

    pub fn record_message(&mut self, text: &str) -> CoreResult<()> {
        let message = SessionMessage {
            seq: self.next_seq,
            timestamp: now_rfc3339(),
            role: "user".to_string(),
            text: text.to_string(),
        };
        self.next_seq = self.next_seq.saturating_add(1);
        self.messages.push(message);
        Ok(())
    }

    pub fn context(&self, limit: Option<usize>) -> CoreResult<SessionContext> {
        self.context_with_id(Some(&self.session_id), limit)
    }

    pub fn context_with_id(
        &self,
        session_id: Option<&str>,
        limit: Option<usize>,
    ) -> CoreResult<SessionContext> {
        if let Some(id) = session_id {
            if id != self.session_id {
                return Err(CoreError::InvalidInput("session not found".to_string()));
            }
        }
        let cap = limit.unwrap_or(DEFAULT_CONTEXT_LIMIT);
        let messages = if self.messages.len() > cap {
            self.messages[self.messages.len() - cap..].to_vec()
        } else {
            self.messages.clone()
        };

        Ok(SessionContext {
            workspace_id: self.workspace.config.workspace_id.clone(),
            session_id: self.session_id.clone(),
            started_at: self.started_at,
            ended_at: self.ended_at,
            messages,
        })
    }

    pub fn open_application(&mut self, app_id: &str) {
        self.application_cache
            .open_application(app_id, now_secs());
    }

    pub fn close_application(&mut self, app_id: &str) {
        self.application_cache.close_application(app_id);
    }

    pub fn destroy(&mut self) -> CoreResult<()> {
        self.ended_at = Some(now_secs());
        self.application_cache = ApplicationCache::new(0, 1);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn session_records_messages() {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::new(dir.path()).expect("workspace"));
        let mut session = Session::new(workspace).expect("session");
        session.record_message("hello").expect("record");
        let ctx = session.context(None).expect("context");
        assert_eq!(ctx.messages.len(), 1);
        assert_eq!(ctx.messages[0].text, "hello");
    }
}
