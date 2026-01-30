use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};
use crate::session::application_cache::ApplicationCache;
use crate::utils::time::now_secs;
use crate::workspace::WorkspaceInstance;

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

    pub fn activate_application(&mut self, app_id: &str) {
        self.application_cache
            .add(app_id, now_secs());
    }

    pub fn active_application_ids(&self) -> Vec<String> {
        self.application_cache.list_applications()
    }

    pub async fn destroy(&mut self) -> CoreResult<()> {
        self.ended_at = Some(now_secs());
        self.application_cache = ApplicationCache::new(0, 1);
        self.persist_info().await?;
        Ok(())
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
