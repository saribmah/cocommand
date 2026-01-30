use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::server::ServerState;
use crate::session::SessionContext;

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
}

#[derive(Debug, Serialize)]
pub struct RecordMessageResponse {
    pub context: ApiSessionContext,
    pub reply: String,
}

pub(crate) async fn record_message(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<RecordMessageRequest>,
) -> Result<Json<RecordMessageResponse>, (StatusCode, String)> {
    let (session_id, messages) = state
        .sessions
        .with_session_mut(|session| {
            Box::pin(async move {
                session.record_message(&payload.text).await?;
                let messages = session.messages_for_prompt(None).await?;
                Ok((session.session_id.clone(), messages))
            })
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let reply = state
        .llm
        .generate_reply_parts(&session_id, &messages)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let reply_text = crate::session::session::render_message_text(&reply);
    let ctx = state
        .sessions
        .with_session_mut(|session| {
            Box::pin(async move {
                if session.session_id != session_id {
                    return Err(crate::error::CoreError::InvalidInput(
                        "session not found".to_string(),
                    ));
                }
                session.record_assistant_parts(reply).await?;
                session.context(None).await
            })
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(RecordMessageResponse {
        context: to_api_context(ctx),
        reply: reply_text,
    }))
}

pub(crate) async fn session_context(
    State(state): State<Arc<ServerState>>,
    Query(params): Query<SessionContextQuery>,
) -> Result<Json<ApiSessionContext>, (StatusCode, String)> {
    let ctx = state
        .sessions
        .with_session_mut(|session| {
            Box::pin(async move {
                session
                    .context_with_id(params.session_id.as_deref(), params.limit)
                    .await
            })
        })
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(to_api_context(ctx)))
}

fn to_api_context(ctx: SessionContext) -> ApiSessionContext {
    ApiSessionContext {
        workspace_id: ctx.workspace_id,
        session_id: ctx.session_id,
        started_at: ctx.started_at,
        ended_at: ctx.ended_at,
    }
}
