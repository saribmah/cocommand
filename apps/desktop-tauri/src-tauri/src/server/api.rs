use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::applications;
use super::state::AppState;

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

async fn execute(
    State(_state): State<AppState>,
    Json(_request): Json<ExecuteRequest>,
) -> Json<ExecuteResponse> {
    Json(ExecuteResponse {
        status: "ok".to_string(),
        message: Some("Executor stub: dispatch tool here.".to_string()),
    })
}
