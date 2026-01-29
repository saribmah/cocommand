use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cocommand::server::Server;
use tauri::Manager;
use tokio::time::{sleep, Duration};

/// Shared application state for workspace/server lifecycle.
pub struct AppState {
    pub workspace_dir: Arc<Mutex<PathBuf>>,
    pub server: Arc<Mutex<Server>>,
}

impl AppState {
    pub fn server_addr(&self) -> String {
        self.server
            .lock()
            .map(|handle| handle.addr().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    }

    pub fn workspace_dir(&self) -> PathBuf {
        self.workspace_dir
            .lock()
            .map(|path| path.clone())
            .unwrap_or_else(|_| PathBuf::new())
    }
}

impl AppState {
    pub fn new(workspace_dir: PathBuf, server: Server) -> Result<Self, String> {
        Ok(Self {
            workspace_dir: Arc::new(Mutex::new(workspace_dir)),
            server: Arc::new(Mutex::new(server)),
        })
    }

    pub fn set_workspace_dir(&self, workspace_dir: PathBuf) -> Result<(), String> {
        let mut guard = self
            .workspace_dir
            .lock()
            .map_err(|_| "workspace_dir lock poisoned".to_string())?;
        *guard = workspace_dir;
        Ok(())
    }

    pub fn replace_server(&self, server: Server) -> Result<(), String> {
        let mut guard = self
            .server
            .lock()
            .map_err(|_| "server lock poisoned".to_string())?;
        guard.shutdown()?;
        *guard = server;
        Ok(())
    }
}

pub async fn start_server_with_retry(
    workspace_dir: PathBuf,
    attempts: usize,
    delay_ms: u64,
) -> Result<Server, String> {
    let mut last_error: Option<String> = None;
    for attempt in 0..attempts {
        match Server::new(workspace_dir.clone()).await {
            Ok(handle) => return Ok(handle),
            Err(error) => {
                last_error = Some(error);
                if attempt + 1 < attempts {
                    sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }
    Err(last_error.unwrap_or_else(|| "failed to start server".to_string()))
}

pub fn resolve_workspace_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let base_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let workspace_dir = base_dir.join("workspace");
    std::fs::create_dir_all(&workspace_dir).map_err(|error| error.to_string())?;
    Ok(workspace_dir)
}
