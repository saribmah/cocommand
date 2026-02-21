use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::command::session_message::SessionCommandInputPart;
use crate::message::Message;
use crate::server::error::{ApiError, ApiErrorResponse};
use crate::server::ServerState;
use crate::session::SessionContext;

#[derive(Debug, Deserialize, ToSchema)]
pub struct RecordMessageRequest {
    pub parts: Vec<SessionCommandInputPart>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct SessionContextQuery {
    pub session_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiSessionContext {
    pub workspace_id: String,
    pub session_id: String,
    pub started_at: u64,
    pub ended_at: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EnqueueMessageResponse {
    pub context: ApiSessionContext,
    pub run_id: String,
    pub accepted_at: u64,
}

#[utoipa::path(
    get,
    path = "/session/command",
    tag = "sessions",
    responses(
        (status = 200, description = "Current session message history", body = [Message]),
        (status = 500, body = ApiErrorResponse),
    ),
    description = "Load message history for the current session."
)]
pub(crate) async fn session_command_history(
    State(state): State<Arc<ServerState>>,
) -> Result<Json<Vec<Message>>, ApiError> {
    let context = state
        .sessions
        .with_session_mut(|session| Box::pin(async move { session.context(None).await }))
        .await
        .map_err(ApiError::from)?;
    let messages = crate::message::message::MessageStorage::load(
        &state.workspace.storage,
        &context.session_id,
    )
    .await
    .map_err(ApiError::from)?;
    Ok(Json(messages))
}

#[utoipa::path(
    post,
    path = "/sessions/command",
    tag = "sessions",
    request_body = RecordMessageRequest,
    responses(
        (status = 200, description = "Message accepted and enqueued", body = EnqueueMessageResponse),
        (status = 400, body = ApiErrorResponse),
    ),
    description = "Enqueue a message for asynchronous session runtime processing."
)]
#[tracing::instrument(skip_all)]
pub(crate) async fn session_command(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<RecordMessageRequest>,
) -> Result<Json<EnqueueMessageResponse>, ApiError> {
    let context = state
        .sessions
        .with_session_mut(|session| Box::pin(async move { session.context(None).await }))
        .await
        .map_err(ApiError::from)?;

    let runtime = state
        .runtime_registry
        .get_or_create(&context.session_id)
        .await;
    let ack = runtime
        .enqueue_user_message(payload.parts)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(EnqueueMessageResponse {
        context: to_api_context(context),
        run_id: ack.run_id,
        accepted_at: ack.accepted_at,
    }))
}

#[utoipa::path(
    get,
    path = "/sessions/context",
    tag = "sessions",
    params(SessionContextQuery),
    responses(
        (status = 200, body = ApiSessionContext),
        (status = 400, body = ApiErrorResponse),
    )
)]
pub(crate) async fn session_context(
    State(state): State<Arc<ServerState>>,
    Query(params): Query<SessionContextQuery>,
) -> Result<Json<ApiSessionContext>, ApiError> {
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
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_api_context_maps_fields() {
        let mapped = to_api_context(SessionContext {
            workspace_id: "workspace-1".to_string(),
            session_id: "session-1".to_string(),
            started_at: 10,
            ended_at: Some(20),
        });

        assert_eq!(mapped.workspace_id, "workspace-1");
        assert_eq!(mapped.session_id, "session-1");
        assert_eq!(mapped.started_at, 10);
        assert_eq!(mapped.ended_at, Some(20));
    }
}
