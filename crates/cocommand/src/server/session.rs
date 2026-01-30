use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::server::ServerState;
use crate::session::{SessionContext, SessionMessage};

#[derive(Debug, Deserialize)]
pub struct RecordMessageRequest {
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct SessionContextQuery {
    pub session_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ApiSessionContext {
    pub workspace_id: String,
    pub session_id: String,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub messages: Vec<SessionMessage>,
}

pub(crate) async fn record_message(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<RecordMessageRequest>,
) -> Result<Json<ApiSessionContext>, (StatusCode, String)> {
    let mut session = state
        .sessions
        .session()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    session
        .record_message(&payload.text)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let reply = state
        .llm
        .generate_reply(&session)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    session
        .record_assistant_message(&reply)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let ctx = session
        .context(None)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(to_api_context(ctx)))
}

pub(crate) async fn session_context(
    State(state): State<Arc<ServerState>>,
    Query(params): Query<SessionContextQuery>,
) -> Result<Json<ApiSessionContext>, (StatusCode, String)> {
    let session = state
        .sessions
        .session()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let ctx = session
        .context_with_id(params.session_id.as_deref(), params.limit)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(to_api_context(ctx)))
}

fn to_api_context(ctx: SessionContext) -> ApiSessionContext {
    ApiSessionContext {
        workspace_id: ctx.workspace_id,
        session_id: ctx.session_id,
        started_at: ctx.started_at,
        ended_at: ctx.ended_at,
        messages: ctx.messages,
    }
}
