use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::message::Message;
use crate::server::ServerState;
use crate::session::SessionContext;
use crate::tool::build_tool_set;

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
    let (session_id, active_apps, ctx) = state
        .sessions
        .with_session_mut(|session| {
            Box::pin(async move {
                let active_apps = session.active_application_ids();
                let ctx = session.context(None).await?;
                Ok((session.session_id.clone(), active_apps, ctx))
            })
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Message::store(&storage, &Message::from_text(&session_id, "user", &payload.text))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let message_history = Message::load(&storage, &session_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let prompt_messages: Vec<llm_kit_provider_utils::message::Message> =
        message_history.iter().filter_map(Message::to_prompt).collect();
    let tools = build_tool_set(
        Arc::new(state.workspace.clone()),
        state.sessions.clone(),
        &session_id,
        &active_apps,
    );
    let reply = state
        .llm
        .generate_reply_parts(&prompt_messages, tools)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let assistant_message = Message::from_stream(&session_id, "assistant", &reply)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Message::store(&storage, &assistant_message)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let reply_text = Message::to_text(&assistant_message);
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
