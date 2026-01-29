use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use tauri::Manager;

use crate::state::resolve_workspace_dir;

const WORKSPACE_PATH_FILE: &str = "workspace-path.json";

#[derive(Debug, Serialize, Deserialize)]
struct WorkspacePathConfig {
    workspace_dir: PathBuf,
}

pub fn load_workspace_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let base_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let path = base_dir.join(WORKSPACE_PATH_FILE);
    if !path.exists() {
        let default_dir = resolve_workspace_dir(app)?;
        save_workspace_dir(app, &default_dir)?;
        return Ok(default_dir);
    }

    let data = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let config: WorkspacePathConfig =
        serde_json::from_str(&data).map_err(|error| error.to_string())?;
    Ok(config.workspace_dir)
}

pub fn save_workspace_dir(app: &tauri::AppHandle, workspace_dir: &PathBuf) -> Result<(), String> {
    let base_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&base_dir).map_err(|error| error.to_string())?;
    let path = base_dir.join(WORKSPACE_PATH_FILE);
    let config = WorkspacePathConfig {
        workspace_dir: workspace_dir.clone(),
    };
    let data = serde_json::to_string_pretty(&config).map_err(|error| error.to_string())?;
    std::fs::write(&path, data).map_err(|error| error.to_string())?;
    Ok(())
}
