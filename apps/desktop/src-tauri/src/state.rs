use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cocommand::server::Server;
use tauri::Manager;
use tokio::time::{sleep, Duration};

#[derive(Clone, Debug)]
pub enum BootStatus {
    Starting,
    Ready,
    Error,
}

#[derive(Clone, Debug)]
pub struct BootState {
    pub status: BootStatus,
    pub error: Option<String>,
}

/// Shared application state for workspace/server lifecycle.
pub struct AppState {
    pub workspace_dir: Arc<Mutex<PathBuf>>,
    pub server: Arc<Mutex<Option<Server>>>,
    pub boot: Arc<Mutex<BootState>>,
}

impl AppState {
    pub fn server_addr(&self) -> Option<String> {
        self.server
            .lock()
            .ok()
            .and_then(|handle| handle.as_ref().map(|server| server.addr().to_string()))
    }

    pub fn workspace_dir(&self) -> PathBuf {
        self.workspace_dir
            .lock()
            .map(|path| path.clone())
            .unwrap_or_else(|_| PathBuf::new())
    }
}

impl AppState {
    pub fn new(workspace_dir: PathBuf) -> Result<Self, String> {
        Ok(Self {
            workspace_dir: Arc::new(Mutex::new(workspace_dir)),
            server: Arc::new(Mutex::new(None)),
            boot: Arc::new(Mutex::new(BootState {
                status: BootStatus::Starting,
                error: None,
            })),
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

    pub fn set_server(&self, server: Server) -> Result<(), String> {
        let mut guard = self
            .server
            .lock()
            .map_err(|_| "server lock poisoned".to_string())?;
        *guard = Some(server);
        Ok(())
    }

    pub fn replace_server(&self, server: Server) -> Result<(), String> {
        let mut guard = self
            .server
            .lock()
            .map_err(|_| "server lock poisoned".to_string())?;
        if let Some(existing) = guard.as_mut() {
            existing.shutdown()?;
        }
        *guard = Some(server);
        Ok(())
    }

    pub fn boot_state(&self) -> BootState {
        self.boot
            .lock()
            .map(|state| state.clone())
            .unwrap_or_else(|_| BootState {
                status: BootStatus::Error,
                error: Some("boot state lock poisoned".to_string()),
            })
    }

    pub fn set_boot_status(&self, status: BootStatus, error: Option<String>) -> Result<(), String> {
        let mut guard = self
            .boot
            .lock()
            .map_err(|_| "boot state lock poisoned".to_string())?;
        *guard = BootState { status, error };
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
