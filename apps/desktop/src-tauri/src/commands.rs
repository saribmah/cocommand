use serde::Serialize;
use tauri::State;

use cocommand::CoreResponse;

use crate::state::AppState;

// --- Serializable DTOs ---

#[derive(Serialize)]
pub struct ActionSummaryDto {
    pub id: String,
    pub description: String,
}

#[derive(Serialize)]
pub struct WorkspaceSnapshotDto {
    pub mode: String,
    pub session_id: String,
    pub instance_count: usize,
    pub follow_up_active: bool,
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
pub fn get_workspace_snapshot(state: State<'_, AppState>) -> Result<WorkspaceSnapshotDto, String> {
    let core = state
        .core
        .lock()
        .map_err(|e| format!("lock poisoned: {e}"))?;
    let ws = core.get_workspace_snapshot().map_err(|e| e.to_string())?;
    Ok(WorkspaceSnapshotDto {
        mode: format!("{:?}", ws.mode),
        session_id: ws.session_id.clone(),
        instance_count: ws.instances.len(),
        follow_up_active: ws.follow_up.is_some(),
    })
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
