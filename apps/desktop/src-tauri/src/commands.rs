use serde::Serialize;
use tauri::State;

use cocommand::{CoreResponse, Workspace};

use crate::state::AppState;

// --- Serializable DTOs ---

#[derive(Serialize)]
pub struct ActionSummaryDto {
    pub id: String,
    pub description: String,
}

// --- Tauri invoke handlers ---

#[tauri::command]
pub fn submit_command(text: String, state: State<'_, AppState>) -> Result<CoreResponse, String> {
    let mut core = state
        .core
        .lock()
        .map_err(|e| format!("lock poisoned: {e}"))?;
    core.submit_command(&text).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn confirm_action(
    confirmation_id: String,
    decision: bool,
    state: State<'_, AppState>,
) -> Result<CoreResponse, String> {
    let mut core = state
        .core
        .lock()
        .map_err(|e| format!("lock poisoned: {e}"))?;
    core.confirm_action(&confirmation_id, decision)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_workspace_snapshot(state: State<'_, AppState>) -> Result<Workspace, String> {
    let core = state
        .core
        .lock()
        .map_err(|e| format!("lock poisoned: {e}"))?;
    core.get_workspace_snapshot().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_recent_actions(
    limit: usize,
    state: State<'_, AppState>,
) -> Result<Vec<ActionSummaryDto>, String> {
    let core = state
        .core
        .lock()
        .map_err(|e| format!("lock poisoned: {e}"))?;
    let actions = core.get_recent_actions(limit).map_err(|e| e.to_string())?;
    Ok(actions
        .into_iter()
        .map(|a| ActionSummaryDto {
            id: a.id,
            description: a.description,
        })
        .collect())
}
