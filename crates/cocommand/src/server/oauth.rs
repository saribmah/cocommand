use axum::extract::{Path, Query, State};
use axum::response::Html;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::server::error::{ApiError, ApiErrorResponse};
use crate::server::ServerState;

// ---------- Request / Response types ----------

#[derive(Deserialize, ToSchema)]
pub struct StartFlowRequest {
    pub state: String,
}

#[derive(Serialize, ToSchema)]
pub struct StartFlowResponse {
    pub redirect_uri: String,
}

#[derive(Deserialize, IntoParams)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

#[derive(Deserialize, IntoParams)]
pub struct PollQuery {
    pub state: String,
    #[serde(default = "default_wait")]
    pub wait: u64,
}

fn default_wait() -> u64 {
    25
}

#[derive(Serialize, ToSchema)]
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

#[utoipa::path(
    post,
    path = "/oauth/start",
    tag = "oauth",
    request_body = StartFlowRequest,
    responses(
        (status = 200, body = StartFlowResponse),
        (status = 409, body = ApiErrorResponse),
    )
)]
pub(crate) async fn start_flow(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<StartFlowRequest>,
) -> Result<Json<StartFlowResponse>, ApiError> {
    state
        .oauth
        .register_flow(&payload.state)
        .await
        .map_err(|e| ApiError::conflict(e))?;

    let redirect_uri = format!("http://{}/oauth/callback", state.addr);
    Ok(Json(StartFlowResponse { redirect_uri }))
}

#[utoipa::path(
    get,
    path = "/oauth/callback",
    tag = "oauth",
    params(CallbackQuery),
    responses(
        (status = 200, description = "HTML page confirming authorization success", content_type = "text/html"),
        (status = 400, body = ApiErrorResponse),
    )
)]
pub(crate) async fn callback(
    State(state): State<Arc<ServerState>>,
    Query(params): Query<CallbackQuery>,
) -> Result<Html<&'static str>, ApiError> {
    state
        .oauth
        .complete_flow(&params.state, &params.code)
        .await
        .map_err(|e| ApiError::bad_request(e))?;

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

#[utoipa::path(
    get,
    path = "/oauth/poll",
    tag = "oauth",
    params(PollQuery),
    responses(
        (status = 200, body = PollResponse),
        (status = 400, body = ApiErrorResponse),
    )
)]
pub(crate) async fn poll_flow(
    State(state): State<Arc<ServerState>>,
    Query(params): Query<PollQuery>,
) -> Result<Json<PollResponse>, ApiError> {
    let wait = params.wait.min(30);
    let result = state
        .oauth
        .poll_flow(&params.state, wait)
        .await
        .map_err(|e| ApiError::bad_request(e))?;

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

#[utoipa::path(
    post,
    path = "/oauth/tokens/{ext}/{provider}",
    tag = "oauth",
    params(
        ("ext" = String, Path, description = "Extension identifier"),
        ("provider" = String, Path, description = "OAuth provider name"),
    ),
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Tokens stored successfully"),
        (status = 500, body = ApiErrorResponse),
    )
)]
pub(crate) async fn set_tokens(
    State(state): State<Arc<ServerState>>,
    Path(path): Path<TokenPath>,
    Json(tokens): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .workspace
        .storage
        .write(&["oauth", &path.ext, &path.provider], &tokens)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

#[utoipa::path(
    get,
    path = "/oauth/tokens/{ext}/{provider}",
    tag = "oauth",
    params(
        ("ext" = String, Path, description = "Extension identifier"),
        ("provider" = String, Path, description = "OAuth provider name"),
    ),
    responses(
        (status = 200, description = "Stored token JSON"),
        (status = 404, body = ApiErrorResponse),
        (status = 500, body = ApiErrorResponse),
    )
)]
pub(crate) async fn get_tokens(
    State(state): State<Arc<ServerState>>,
    Path(path): Path<TokenPath>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let value = state
        .workspace
        .storage
        .read(&["oauth", &path.ext, &path.provider])
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    match value {
        Some(v) => Ok(Json(v)),
        None => Err(ApiError::not_found("no tokens found")),
    }
}

#[utoipa::path(
    delete,
    path = "/oauth/tokens/{ext}/{provider}",
    tag = "oauth",
    params(
        ("ext" = String, Path, description = "Extension identifier"),
        ("provider" = String, Path, description = "OAuth provider name"),
    ),
    responses(
        (status = 200, description = "Tokens deleted successfully"),
        (status = 500, body = ApiErrorResponse),
    )
)]
pub(crate) async fn delete_tokens(
    State(state): State<Arc<ServerState>>,
    Path(path): Path<TokenPath>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .workspace
        .storage
        .delete(&["oauth", &path.ext, &path.provider])
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({ "ok": true })))
}
