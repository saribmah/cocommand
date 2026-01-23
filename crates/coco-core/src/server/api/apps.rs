//! Application route handlers.
//!
//! This module handles endpoints related to applications and tools.

use axum::{extract::State, Json};
use std::collections::HashSet;

use crate::applications;

use super::super::state::AppState;
use super::types::{ExecuteRequest, ExecuteResponse};

/// Handle the /health GET endpoint.
pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

/// Handle the /apps GET endpoint.
///
/// Returns all available applications.
pub async fn list_apps(
    State(_state): State<AppState>,
) -> Json<Vec<applications::ApplicationDefinition>> {
    Json(applications::all_apps())
}

/// Handle the /window/apps GET endpoint.
///
/// Returns all available applications (alias for /apps).
pub async fn window_apps(
    State(_state): State<AppState>,
) -> Json<Vec<applications::ApplicationDefinition>> {
    Json(applications::all_apps())
}

/// Handle the /tools GET endpoint.
///
/// Returns tools for currently open applications only.
pub async fn list_tools(State(state): State<AppState>) -> Json<Vec<applications::ToolDefinition>> {
    let workspace = state.store.load();
    if let Ok(workspace) = workspace {
        let open_ids: HashSet<String> = workspace
            .open_apps
            .iter()
            .map(|app| app.id.clone())
            .collect();
        let tools = applications::all_apps()
            .into_iter()
            .filter(|app| open_ids.contains(&app.id))
            .flat_map(|app| app.tools)
            .collect();
        return Json(tools);
    }
    Json(Vec::new())
}

/// Handle the /execute POST endpoint.
///
/// Executes a tool directly (requires the app to be open).
/// Respects workspace lifecycle: archived workspaces require restore before execution.
pub async fn execute(
    State(state): State<AppState>,
    Json(request): Json<ExecuteRequest>,
) -> Json<ExecuteResponse> {
    let workspace = match state.store.load() {
        Ok(workspace) => workspace,
        Err(error) => return Json(ExecuteResponse::error(error)),
    };

    // Check if workspace is archived - block execution
    if state.workspace.is_archived(&workspace) {
        return Json(ExecuteResponse::error(
            "Workspace is archived. Use window.restore_workspace to recover.",
        ));
    }

    // Extract app ID from tool ID (format: appid_action)
    let app_id = match applications::app_id_from_tool(&request.tool_id) {
        Some(id) if !id.is_empty() => id,
        _ => {
            return Json(ExecuteResponse::error(format!(
                "Invalid tool id: {}",
                request.tool_id
            )))
        }
    };

    // Check if app is open
    let app_open = workspace.open_apps.iter().any(|app| app.id == app_id);
    if !app_open {
        return Json(ExecuteResponse::error(format!("App not open: {}", app_id)));
    }

    // Check if tool exists for the app
    let tool_allowed = applications::app_by_id(app_id)
        .map(|app| app.tools.iter().any(|tool| tool.id == request.tool_id))
        .unwrap_or(false);
    if !tool_allowed {
        return Json(ExecuteResponse::error(format!(
            "Unknown tool: {}",
            request.tool_id
        )));
    }

    // Execute the tool
    match applications::execute_tool(&request.tool_id, request.inputs) {
        Some(result) => Json(ExecuteResponse {
            status: result.status,
            message: Some(result.message),
        }),
        None => Json(ExecuteResponse::error(format!(
            "Unknown tool: {}",
            request.tool_id
        ))),
    }
}
