use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::error::{CoreError, CoreResult};
use crate::workspace::config::{load_or_create_workspace_config, WorkspaceConfig};
use crate::workspace::window_cache::WindowCache;

#[derive(Debug, Clone)]
pub struct WorkspaceInstance {
    pub workspace_dir: PathBuf,
    pub config: WorkspaceConfig,
    window_cache_state: Arc<Mutex<WindowCacheState>>,
}

#[derive(Debug, Clone)]
struct WindowCacheState {
    session_id: String,
    cache: WindowCache,
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
        let max_windows = config.preferences.window_cache.max_windows;
        let window_cache = WindowCache::new(max_windows, ttl);
        Ok(Self {
            workspace_dir: workspace_dir.to_path_buf(),
            config,
            window_cache_state: Arc::new(Mutex::new(WindowCacheState {
                session_id: String::new(),
                cache: window_cache,
            })),
        })
    }

    pub fn sessions_path(&self) -> PathBuf {
        self.workspace_dir.join("sessions.json")
    }

    pub fn open_window(&self, session_id: &str, window_id: &str, opened_at: u64) {
        let cache = self.ensure_window_cache(session_id);
        cache.open_window(window_id, opened_at);
    }

    pub fn close_window(&self, session_id: &str, window_id: &str) {
        let cache = self.ensure_window_cache(session_id);
        cache.close_window(window_id);
    }

    fn ensure_window_cache(&self, session_id: &str) -> WindowCache {
        let mut guard = self
            .window_cache_state
            .lock()
            .expect("window cache lock");
        if guard.session_id != session_id {
            let ttl = self.config.preferences.session.duration_seconds;
            let max_windows = self.config.preferences.window_cache.max_windows;
            guard.cache = WindowCache::new(max_windows, ttl);
            guard.session_id = session_id.to_string();
        }
        guard.cache.clone()
    }
}
