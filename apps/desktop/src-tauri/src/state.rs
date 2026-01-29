use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cocommand::server::ServerHandle;
use tauri::Manager;
use tokio::time::{sleep, Duration};

/// Shared application state for workspace/server lifecycle.
pub struct AppState {
    pub workspace_dir: PathBuf,
    pub server_handle: Arc<Mutex<ServerHandle>>,
}

impl AppState {
    pub fn server_addr(&self) -> String {
        self.server_handle
            .lock()
            .map(|handle| handle.addr().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    }

    pub fn workspace_dir(&self) -> &PathBuf {
        &self.workspace_dir
    }

}

impl AppState {
    pub fn new(workspace_dir: PathBuf, server_handle: ServerHandle) -> Result<Self, String> {
        Ok(Self {
            workspace_dir,
            server_handle: Arc::new(Mutex::new(server_handle)),
        })
    }
}

pub async fn start_server_with_retry(
    workspace_dir: PathBuf,
    attempts: usize,
    delay_ms: u64,
) -> Result<ServerHandle, String> {
    let mut last_error: Option<String> = None;
    for attempt in 0..attempts {
        match cocommand::server::start(workspace_dir.clone()).await {
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
