use serde::Serialize;
use tauri::State;

use cocommand::{ConfirmationDecision, CoreResponse};

use crate::state::AppState;

// --- Serializable DTOs (Core types lack Serialize) ---

#[derive(Serialize)]
pub struct RoutedCandidateDto {
    pub app_id: String,
    pub score: f64,
    pub explanation: String,
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum CoreResponseDto {
    Routed {
        candidates: Vec<RoutedCandidateDto>,
        follow_up_active: bool,
    },
    ClarificationNeeded {
        message: String,
    },
}

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

// --- Conversion helpers ---

fn core_response_to_dto(resp: CoreResponse) -> CoreResponseDto {
    match resp {
        CoreResponse::Routed {
            candidates,
            follow_up_active,
        } => CoreResponseDto::Routed {
            candidates: candidates
                .into_iter()
                .map(|c| RoutedCandidateDto {
                    app_id: c.app_id,
                    score: c.score,
                    explanation: c.explanation,
                })
                .collect(),
            follow_up_active,
        },
        CoreResponse::ClarificationNeeded { message } => {
            CoreResponseDto::ClarificationNeeded { message }
        }
    }
}

// --- Tauri invoke handlers ---

#[tauri::command]
pub fn submit_command(text: String, state: State<'_, AppState>) -> Result<CoreResponseDto, String> {
    let mut core = state
        .core
        .lock()
        .map_err(|e| format!("lock poisoned: {e}"))?;
    let response = core.submit_command(&text).map_err(|e| e.to_string())?;
    Ok(core_response_to_dto(response))
}

#[tauri::command]
pub fn confirm_action(
    confirmation_id: String,
    decision: String,
    state: State<'_, AppState>,
) -> Result<CoreResponseDto, String> {
    let decision = match decision.as_str() {
        "approve" => ConfirmationDecision::Approve,
        "deny" => ConfirmationDecision::Deny,
        _ => return Err("decision must be \"approve\" or \"deny\"".into()),
    };
    let core = state
        .core
        .lock()
        .map_err(|e| format!("lock poisoned: {e}"))?;
    let response = core
        .confirm_action(&confirmation_id, decision)
        .map_err(|e| e.to_string())?;
    Ok(core_response_to_dto(response))
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
