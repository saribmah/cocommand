use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::message::{render_message_text, Message, MessageInfo, MessageWithParts};
use crate::server::ServerState;
use crate::session::SessionContext;
use crate::llm::tools::build_tool_set;
use crate::utils::time::now_rfc3339;
use uuid::Uuid;

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
    let storage = state.workspace.storage.clone();
    let (session_id, active_apps) = state
        .sessions
        .with_session_mut(|session| {
            Box::pin(async move {
                let active_apps = session.active_application_ids();
                Ok((session.session_id.clone(), active_apps))
            })
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let user_message = Message::from_text(&session_id, "user", &payload.text);
    Message::store(&storage, &user_message)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let message_history = Message::load(&storage, &session_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let tools = build_tool_set(
        Arc::new(state.workspace.clone()),
        state.sessions.clone(),
        &session_id,
        &active_apps,
    );
    let reply = state
        .llm
        .generate_reply_parts(&message_history, tools)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let reply_text = render_message_text(&reply);
    let ctx = state
        .sessions
        .with_session_mut(|session| {
            let storage = storage.clone();
            let reply_parts = reply.clone();
            Box::pin(async move {
                if session.session_id != session_id {
                    return Err(crate::error::CoreError::InvalidInput(
                        "session not found".to_string(),
                    ));
                }
                let timestamp = now_rfc3339();
                let assistant_message = MessageWithParts {
                    info: MessageInfo {
                        id: Uuid::now_v7().to_string(),
                        session_id: session_id.clone(),
                        role: "assistant".to_string(),
                        created_at: timestamp.clone(),
                        updated_at: timestamp,
                    },
                    parts: reply_parts,
                };
                Message::store(&storage, &assistant_message).await?;
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
