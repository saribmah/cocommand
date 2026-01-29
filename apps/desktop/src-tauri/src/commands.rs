use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::state::{start_server_with_retry, AppState};
use crate::workspace_path::save_workspace_dir;

#[tauri::command]
pub fn get_workspace_dir_cmd(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.workspace_dir().display().to_string())
}

#[tauri::command]
pub async fn set_workspace_dir_cmd(
    workspace_dir: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let new_dir = PathBuf::from(workspace_dir);
    std::fs::create_dir_all(&new_dir).map_err(|error| error.to_string())?;
    save_workspace_dir(&app, &new_dir)?;

    let new_handle = start_server_with_retry(new_dir.clone(), 3, 200).await?;
    state.replace_server_handle(new_handle)?;
    state.set_workspace_dir(new_dir)?;

    Ok(state.server_addr())
}
