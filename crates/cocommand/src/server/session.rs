use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::any::Any;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;

use crate::command::session_message::{
    run_session_command, SessionCommandInput, SessionCommandContextEvent,
    SessionCommandPartUpdatedEvent,
};
use crate::message::parts::MessagePart;
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
    pub reply_parts: Vec<MessagePart>,
}

pub(crate) async fn session_command(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<RecordMessageRequest>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let request_id = Uuid::now_v7().to_string();
    let text = payload.text;

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
                SessionCommandInput { request_id, text },
            )
            .await;
            let payload = result.ok().map(|output| RecordMessageResponse {
                context: to_api_context(output.context),
                reply_parts: output.reply_parts,
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
                                    "reply_parts": result.reply_parts,
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
                        let mapped = match map_bus_event_to_sse_payload(bus_event.as_ref(), &request_id) {
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

struct BusSseEvent {
    event: &'static str,
    payload: serde_json::Value,
}

fn map_bus_event_to_sse_payload(
    event: &(dyn Any + Send + Sync),
    request_id: &str,
) -> Option<BusSseEvent> {
    if let Some(event) = event.downcast_ref::<SessionCommandPartUpdatedEvent>() {
        if event.request_id != request_id {
            return None;
        }
        return Some(BusSseEvent {
            event: "part.updated",
            payload: json!({
                "part_id": &event.part_id,
                "part": &event.part,
            }),
        });
    }

    if let Some(event) = event.downcast_ref::<SessionCommandContextEvent>() {
        if event.request_id != request_id {
            return None;
        }
        return Some(BusSseEvent {
            event: "context",
            payload: json!({
                "context": to_api_context(event.context.clone()),
            }),
        });
    }

    None
}

fn to_api_context(ctx: SessionContext) -> ApiSessionContext {
    ApiSessionContext {
        workspace_id: ctx.workspace_id,
        session_id: ctx.session_id,
        started_at: ctx.started_at,
        ended_at: ctx.ended_at,
    }
}
