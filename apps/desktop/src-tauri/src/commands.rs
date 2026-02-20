use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::state::{start_server_with_retry, AppState, BootStatus};
use crate::workspace_path::save_workspace_dir;

#[tauri::command]
pub fn get_workspace_dir_cmd(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.workspace_dir().display().to_string())
}

#[tauri::command]
pub fn get_server_info_cmd(state: State<'_, AppState>) -> Result<ServerStatusDto, String> {
    let boot = state.boot_state();
    let status = match boot.status {
        BootStatus::Starting => "starting",
        BootStatus::Ready => "ready",
        BootStatus::Error => "error",
    };
    Ok(ServerStatusDto {
        status: status.to_string(),
        addr: state.server_addr(),
        workspace_dir: state.workspace_dir().display().to_string(),
        error: boot.error,
    })
}

#[tauri::command]
pub async fn set_workspace_dir_cmd(
    workspace_dir: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let new_dir = PathBuf::from(workspace_dir);
    std::fs::create_dir_all(&new_dir).map_err(|error| error.to_string())?;
    let new_server = start_server_with_retry(new_dir.clone(), 3, 200).await?;
    state.replace_server(new_server)?;
    state.set_workspace_dir(new_dir)?;
    save_workspace_dir(&app, &state.workspace_dir())?;
    let _ = state.set_boot_status(BootStatus::Ready, None);

    Ok(state.server_addr().unwrap_or_else(|| "unknown".to_string()))
}

#[derive(serde::Serialize)]
pub struct ServerStatusDto {
    pub status: String,
    pub addr: Option<String>,
    pub workspace_dir: String,
    pub error: Option<String>,
}
