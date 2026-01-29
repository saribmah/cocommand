use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::error::{CoreError, CoreResult};
use crate::workspace::application_cache::ApplicationCache;
use crate::workspace::config::{load_or_create_workspace_config, WorkspaceConfig};

#[derive(Debug, Clone)]
pub struct WorkspaceInstance {
    pub workspace_dir: PathBuf,
    pub config: WorkspaceConfig,
    application_cache_state: Arc<Mutex<ApplicationCacheState>>,
}

#[derive(Debug, Clone)]
struct ApplicationCacheState {
    session_id: String,
    cache: ApplicationCache,
}

impl WorkspaceInstance {
    pub fn load(workspace_dir: &Path) -> CoreResult<Self> {
        if !workspace_dir.exists() {
            std::fs::create_dir_all(workspace_dir).map_err(|error| {
                CoreError::Internal(format!(
                    "failed to create workspace directory {}: {error}",
                    workspace_dir.display()
                ))
            })?;
        }
        let config = load_or_create_workspace_config(workspace_dir)?;
        let ttl = config.preferences.session.duration_seconds;
        let max_apps = config.preferences.application_cache.max_applications;
        let application_cache = ApplicationCache::new(max_apps, ttl);
        Ok(Self {
            workspace_dir: workspace_dir.to_path_buf(),
            config,
            application_cache_state: Arc::new(Mutex::new(ApplicationCacheState {
                session_id: String::new(),
                cache: application_cache,
            })),
        })
    }

    pub fn sessions_path(&self) -> PathBuf {
        self.workspace_dir.join("sessions.json")
    }

    pub fn open_application(&self, session_id: &str, app_id: &str, opened_at: u64) {
        let cache = self.ensure_application_cache(session_id);
        cache.open_application(app_id, opened_at);
    }

    pub fn close_application(&self, session_id: &str, app_id: &str) {
        let cache = self.ensure_application_cache(session_id);
        cache.close_application(app_id);
    }

    fn ensure_application_cache(&self, session_id: &str) -> ApplicationCache {
        let mut guard = self
            .application_cache_state
            .lock()
            .expect("application cache lock");
        if guard.session_id != session_id {
            let ttl = self.config.preferences.session.duration_seconds;
            let max_apps = self.config.preferences.application_cache.max_applications;
            guard.cache = ApplicationCache::new(max_apps, ttl);
            guard.session_id = session_id.to_string();
        }
        guard.cache.clone()
    }
}
