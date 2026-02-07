use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};
use crate::session::extension_cache::ExtensionCache;
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
    extension_cache: ExtensionCache,
}

impl Session {
    pub async fn new(workspace: Arc<WorkspaceInstance>) -> CoreResult<Self> {
        let (ttl, max_apps) = {
            let config = workspace.config.read().await;
            (
                config.preferences.session.duration_seconds,
                config.preferences.extension_cache.max_extensions,
            )
        };
        let cache = ExtensionCache::new(max_apps, ttl);
        let session = Self {
            workspace,
            session_id: Uuid::now_v7().to_string(),
            started_at: now_secs(),
            ended_at: None,
            extension_cache: cache,
        };
        session.persist_info().await?;
        Ok(session)
    }

    pub async fn from_info(
        workspace: Arc<WorkspaceInstance>,
        info: SessionInfo,
    ) -> CoreResult<Self> {
        let (ttl, max_apps) = {
            let config = workspace.config.read().await;
            (
                config.preferences.session.duration_seconds,
                config.preferences.extension_cache.max_extensions,
            )
        };
        let cache = ExtensionCache::new(max_apps, ttl);
        Ok(Self {
            workspace,
            session_id: info.id,
            started_at: info.started_at,
            ended_at: info.ended_at,
            extension_cache: cache,
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
        let workspace_id = {
            let config = self.workspace.config.read().await;
            config.workspace_id.clone()
        };
        Ok(SessionContext {
            workspace_id,
            session_id: self.session_id.clone(),
            started_at: self.started_at,
            ended_at: self.ended_at,
        })
    }

    pub fn activate_extension(&mut self, app_id: &str) {
        self.extension_cache.add(app_id, now_secs());
    }

    pub fn active_extension_ids(&self) -> Vec<String> {
        self.extension_cache.list_extensions()
    }

    pub async fn destroy(&mut self) -> CoreResult<()> {
        self.ended_at = Some(now_secs());
        self.extension_cache = ExtensionCache::new(0, 1);
        self.persist_info().await?;
        Ok(())
    }

    async fn persist_info(&self) -> CoreResult<()> {
        let info = SessionInfo {
            id: self.session_id.clone(),
            workspace_id: {
                let config = self.workspace.config.read().await;
                config.workspace_id.clone()
            },
            started_at: self.started_at,
            ended_at: self.ended_at,
        };
        let value = serde_json::to_value(info).map_err(|error| {
            CoreError::Internal(format!("failed to serialize session info: {error}"))
        })?;
        let workspace_id = {
            let config = self.workspace.config.read().await;
            config.workspace_id.clone()
        };
        self.workspace
            .storage
            .write(&["session", &workspace_id, &self.session_id], &value)
            .await
    }
}
