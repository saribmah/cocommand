use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::applications;
use super::state::AppState;
use crate::commands::intake as command_intake;
use crate::llm::selector;

#[derive(Deserialize)]
struct PlanRequest {
    text: String,
    app_id: Option<String>,
}

#[derive(Serialize)]
struct PlanResponse {
    status: String,
    message: Option<String>,
}

#[derive(Deserialize)]
struct CommandSubmitRequest {
    text: String,
}

#[derive(Serialize)]
struct CommandSubmitResponse {
    status: String,
    command: Option<command_intake::CommandInput>,
    app_id: Option<String>,
    tool_id: Option<String>,
    message: Option<String>,
}

#[derive(Deserialize)]
struct ExecuteRequest {
    tool_id: String,
    inputs: serde_json::Value,
}

#[derive(Serialize)]
struct ExecuteResponse {
    status: String,
    message: Option<String>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/apps", get(apps))
        .route("/tools", get(tools))
        .route("/command", post(command))
        .route("/plan", post(plan))
        .route("/execute", post(execute))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn tools(State(_state): State<AppState>) -> Json<Vec<applications::ToolDefinition>> {
    Json(applications::all_tools())
}

async fn apps(
    State(_state): State<AppState>,
) -> Json<Vec<applications::ApplicationDefinition>> {
    Json(applications::all_apps())
}

async fn plan(State(_state): State<AppState>, Json(request): Json<PlanRequest>) -> Json<PlanResponse> {
    if request.text.trim().is_empty() {
        return Json(PlanResponse {
            status: "empty".to_string(),
            message: Some("Type a command to get started.".to_string()),
        });
    }

    Json(PlanResponse {
        status: "ok".to_string(),
        message: Some("Planner stub: connect LLM here.".to_string()),
    })
}

async fn command(
    State(_state): State<AppState>,
    Json(request): Json<CommandSubmitRequest>,
) -> Json<CommandSubmitResponse> {
    if request.text.trim().is_empty() {
        return Json(CommandSubmitResponse {
            status: "empty".to_string(),
            command: None,
            app_id: None,
            tool_id: None,
            message: Some("Type a command to get started.".to_string()),
        });
    }

    let command = command_intake::normalize(command_intake::CommandRequest {
        text: request.text,
        source: Some("ui".to_string()),
    });

    let apps = applications::all_apps();
    let app_selection = selector::select_application(&command.text, &apps);
    let (app_id, tool_id) = match app_selection {
        Some(selection) => {
            let tools = applications::app_by_id(&selection.app_id)
                .map(|app| app.tools)
                .unwrap_or_default();
            let tool_selection = selector::select_tool(&command.text, &tools);
            (
                Some(selection.app_id),
                tool_selection.map(|tool| tool.tool_id),
            )
        }
        None => (None, None),
    };

    Json(CommandSubmitResponse {
        status: "ok".to_string(),
        command: Some(command),
        app_id,
        tool_id,
        message: None,
    })
}

async fn execute(
    State(_state): State<AppState>,
    Json(request): Json<ExecuteRequest>,
) -> Json<ExecuteResponse> {
    match applications::execute_tool(&request.tool_id, request.inputs) {
        Some(result) => Json(ExecuteResponse {
            status: result.status,
            message: Some(result.message),
        }),
        None => Json(ExecuteResponse {
            status: "error".to_string(),
            message: Some(format!("Unknown tool: {}", request.tool_id)),
        }),
    }
}
