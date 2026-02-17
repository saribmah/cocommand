use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Html;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::server::ServerState;

// ---------- Request / Response types ----------

#[derive(Deserialize)]
pub struct StartFlowRequest {
    pub state: String,
}

#[derive(Serialize)]
pub struct StartFlowResponse {
    pub redirect_uri: String,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

#[derive(Deserialize)]
pub struct PollQuery {
    pub state: String,
    #[serde(default = "default_wait")]
    pub wait: u64,
}

fn default_wait() -> u64 {
    25
}

#[derive(Serialize)]
pub struct PollResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_code: Option<String>,
}

#[derive(Deserialize)]
pub struct TokenPath {
    pub ext: String,
    pub provider: String,
}

// ---------- Handlers ----------

pub(crate) async fn start_flow(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<StartFlowRequest>,
) -> Result<Json<StartFlowResponse>, (StatusCode, String)> {
    state
        .oauth
        .register_flow(&payload.state)
        .await
        .map_err(|e| (StatusCode::CONFLICT, e))?;

    let redirect_uri = format!("http://{}/oauth/callback", state.addr);
    Ok(Json(StartFlowResponse { redirect_uri }))
}

pub(crate) async fn callback(
    State(state): State<Arc<ServerState>>,
    Query(params): Query<CallbackQuery>,
) -> Result<Html<&'static str>, (StatusCode, String)> {
    state
        .oauth
        .complete_flow(&params.state, &params.code)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    Ok(Html(
        r#"<!DOCTYPE html>
<html>
<head><title>Authorization Complete</title></head>
<body style="font-family:system-ui,sans-serif;display:flex;justify-content:center;align-items:center;height:100vh;margin:0">
<div style="text-align:center">
<h2>Authorization successful</h2>
<p>You can close this window.</p>
<script>setTimeout(()=>window.close(),1500)</script>
</div>
</body>
</html>"#,
    ))
}

pub(crate) async fn poll_flow(
    State(state): State<Arc<ServerState>>,
    Query(params): Query<PollQuery>,
) -> Result<Json<PollResponse>, (StatusCode, String)> {
    let wait = params.wait.min(30);
    let result = state
        .oauth
        .poll_flow(&params.state, wait)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    match result {
        Some(code) => Ok(Json(PollResponse {
            status: "completed".to_string(),
            authorization_code: Some(code),
        })),
        None => Ok(Json(PollResponse {
            status: "pending".to_string(),
            authorization_code: None,
        })),
    }
}

pub(crate) async fn set_tokens(
    State(state): State<Arc<ServerState>>,
    Path(path): Path<TokenPath>,
    Json(tokens): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    state
        .workspace
        .storage
        .write(&["oauth", &path.ext, &path.provider], &tokens)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub(crate) async fn get_tokens(
    State(state): State<Arc<ServerState>>,
    Path(path): Path<TokenPath>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let value = state
        .workspace
        .storage
        .read(&["oauth", &path.ext, &path.provider])
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match value {
        Some(v) => Ok(Json(v)),
        None => Err((StatusCode::NOT_FOUND, "no tokens found".to_string())),
    }
}

pub(crate) async fn delete_tokens(
    State(state): State<Arc<ServerState>>,
    Path(path): Path<TokenPath>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    state
        .workspace
        .storage
        .delete(&["oauth", &path.ext, &path.provider])
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({ "ok": true })))
}
