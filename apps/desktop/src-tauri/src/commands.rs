use tauri::State;

use cocommand::{get_session_context, record_user_message, SessionContext};

use crate::state::AppState;

#[tauri::command]
pub fn record_user_message_cmd(
    text: String,
    state: State<'_, AppState>,
) -> Result<SessionContext, String> {
    record_user_message(state.workspace_dir(), &text).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_session_context_cmd(
    session_id: Option<String>,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<SessionContext, String> {
    let session_id = session_id.as_deref();
    get_session_context(state.workspace_dir(), session_id, limit).map_err(|e| e.to_string())
}
