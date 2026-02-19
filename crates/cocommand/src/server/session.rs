use axum::{
    extract::{Query, State},
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::wrappers::UnboundedReceiverStream;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::command::session_message::{
    run_session_command, SessionCommandInput, SessionCommandInputPart,
};
use crate::event::CoreEvent;
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
pub struct RecordMessageResponse {
    pub context: ApiSessionContext,
    pub messages: Vec<Message>,
}

#[utoipa::path(
    post,
    path = "/sessions/command",
    tag = "sessions",
    request_body = RecordMessageRequest,
    responses(
        (status = 200, description = "SSE stream of session events", content_type = "text/event-stream"),
    ),
    description = "Process a user command. Returns an SSE stream of part.updated, context, and done events."
)]
#[tracing::instrument(skip_all)]
pub(crate) async fn session_command(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<RecordMessageRequest>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let request_id = Uuid::now_v7().to_string();
    let parts = payload.parts;

    let mut bus_rx = state.bus.subscribe();
    let (command_result_tx, mut command_result_rx) =
        tokio::sync::mpsc::unbounded_channel::<Option<RecordMessageResponse>>();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Result<Event, Infallible>>();

    tokio::spawn({
        let state = state.clone();
        let request_id = request_id.clone();
        async move {
            let result = run_session_command(
                state.sessions.clone(),
                state.workspace.clone(),
                &state.llm,
                &state.bus,
                SessionCommandInput { request_id, parts },
            )
            .await;
            let payload = result.ok().map(|output| RecordMessageResponse {
                context: to_api_context(output.context),
                messages: output.messages,
            });
            let _ = command_result_tx.send(payload);
        }
    });

    tokio::spawn({
        let request_id = request_id.clone();
        async move {
            loop {
                tokio::select! {
                    command_result = command_result_rx.recv() => {
                        match command_result {
                            Some(Some(result)) => {
                                let sse_event = Event::default().event("done").data(json!({
                                    "context": result.context,
                                    "messages": result.messages,
                                }).to_string());
                                let _ = tx.send(Ok(sse_event));
                                break;
                            }
                            Some(None) | None => break,
                        }
                    }
                    bus_event = bus_rx.recv() => {
                        let bus_event = match bus_event {
                            Ok(event) => event,
                            Err(_) => break,
                        };
                        let mapped = match map_bus_event_to_sse_payload(&bus_event, &request_id) {
                            Some(mapped) => mapped,
                            None => continue,
                        };

                        let sse_event = Event::default()
                            .event(mapped.event)
                            .data(mapped.payload.to_string());
                        if tx.send(Ok(sse_event)).is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });

    Sse::new(UnboundedReceiverStream::new(rx)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
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

struct BusSseEvent {
    event: &'static str,
    payload: serde_json::Value,
}

fn map_bus_event_to_sse_payload(event: &CoreEvent, request_id: &str) -> Option<BusSseEvent> {
    match event {
        CoreEvent::SessionMessageStarted(e) => {
            if e.request_id != request_id {
                return None;
            }
            Some(BusSseEvent {
                event: "message.started",
                payload: json!({
                    "user_message": &e.user_message,
                    "assistant_message": &e.assistant_message,
                }),
            })
        }
        CoreEvent::SessionPartUpdated(e) => {
            if e.request_id != request_id {
                return None;
            }
            Some(BusSseEvent {
                event: "part.updated",
                payload: json!({
                    "message_id": &e.message_id,
                    "part_id": &e.part_id,
                    "part": &e.part,
                }),
            })
        }
        CoreEvent::SessionContextUpdated(e) => {
            if e.request_id != request_id {
                return None;
            }
            Some(BusSseEvent {
                event: "context",
                payload: json!({
                    "context": to_api_context(e.context.clone()),
                }),
            })
        }
    }
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
    use crate::event::{SessionMessageStartedPayload, SessionPartUpdatedPayload};
    use crate::message::{Message, MessagePart, PartBase, TextPart};

    #[test]
    fn map_bus_event_maps_message_started_payload() {
        let event = CoreEvent::SessionMessageStarted(SessionMessageStartedPayload {
            request_id: "request-1".to_string(),
            user_message: Message::from_text("session-1", "user", "hello"),
            assistant_message: Message::from_parts("session-1", "assistant", Vec::new()),
        });

        let mapped = map_bus_event_to_sse_payload(&event, "request-1").expect("mapped");
        assert_eq!(mapped.event, "message.started");
        assert_eq!(mapped.payload["user_message"]["info"]["role"], "user");
        assert_eq!(
            mapped.payload["assistant_message"]["info"]["role"],
            "assistant"
        );
        assert!(map_bus_event_to_sse_payload(&event, "request-2").is_none());
    }

    #[test]
    fn map_bus_event_maps_part_updated_with_message_id() {
        let event = CoreEvent::SessionPartUpdated(SessionPartUpdatedPayload {
            request_id: "request-1".to_string(),
            session_id: "session-1".to_string(),
            message_id: "message-1".to_string(),
            part_id: "part-1".to_string(),
            part: MessagePart::Text(TextPart {
                base: PartBase {
                    id: "part-1".to_string(),
                    session_id: "session-1".to_string(),
                    message_id: "message-1".to_string(),
                },
                text: "hello".to_string(),
            }),
        });

        let mapped = map_bus_event_to_sse_payload(&event, "request-1").expect("mapped");
        assert_eq!(mapped.event, "part.updated");
        assert_eq!(mapped.payload["message_id"], "message-1");
        assert_eq!(mapped.payload["part_id"], "part-1");
        assert_eq!(mapped.payload["part"]["type"], "text");
    }
}
