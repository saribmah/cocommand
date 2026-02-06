use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::message::Message;
use crate::message::convert::outputs_to_parts;
use crate::message::parts::MessagePart;
use crate::server::ServerState;
use crate::session::SessionContext;
use crate::tool::ToolRegistry;
use llm_kit_core::output::{Output, ReasoningOutput, TextOutput};
use llm_kit_core::stream_text::TextStreamPart;
use llm_kit_core::generate_text::GeneratedFile;

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

pub(crate) async fn record_message_stream(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<RecordMessageRequest>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let state = state.clone();
    let text = payload.text;
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Result<Event, Infallible>>();
    tokio::spawn(async move {
        let send = |event: Event| {
            let _ = tx.send(Ok(event));
        };
        let storage = state.workspace.storage.clone();
        let (session_id, active_apps, ctx) = match state
            .sessions
            .with_session_mut(|session| {
                Box::pin(async move {
                    let active_apps = session.active_extension_ids();
                    let ctx = session.context(None).await?;
                    Ok((session.session_id.clone(), active_apps, ctx))
                })
            })
            .await
        {
            Ok(values) => values,
            Err(error) => {
                send(Event::default().event("error").data(json!({
                    "error": error.to_string()
                }).to_string()));
                return;
            }
        };

        let api_context = to_api_context(ctx);
        send(Event::default().event("context").data(json!({
            "context": api_context
        }).to_string()));

        if let Err(error) = Message::store(&storage, &Message::from_text(&session_id, "user", &text)).await {
            send(Event::default().event("error").data(json!({
                "error": error.to_string()
            }).to_string()));
            return;
        }

        let message_history = match Message::load(&storage, &session_id).await {
            Ok(history) => history,
            Err(error) => {
                send(Event::default().event("error").data(json!({
                    "error": error.to_string()
                }).to_string()));
                return;
            }
        };
        let prompt_messages: Vec<llm_kit_provider_utils::message::Message> = message_history
            .iter()
            .flat_map(Message::to_prompt_messages)
            .collect();
        let tools = ToolRegistry::tools(
            Arc::new(state.workspace.clone()),
            state.sessions.clone(),
            &session_id,
            &active_apps,
        )
        .await;
        let reply = match state
            .llm
            .stream_text(&prompt_messages, tools)
            .await
        {
            Ok(result) => result,
            Err(error) => {
                send(Event::default().event("error").data(json!({
                    "error": error.to_string()
                }).to_string()));
                return;
            }
        };

        let mut outputs: Vec<Output> = Vec::new();
        let mut current_text = String::new();
        let mut current_reasoning = String::new();

        let mut full_stream = reply.full_stream();
        while let Some(part) = full_stream.next().await {
            send(Event::default().event("part").data(json!({
                "part": &part
            }).to_string()));

            match part {
                TextStreamPart::TextDelta { text, .. } => {
                    current_text.push_str(&text);
                }
                TextStreamPart::TextEnd { .. } => {
                    if !current_text.is_empty() {
                        outputs.push(Output::Text(TextOutput::new(current_text.clone())));
                        current_text.clear();
                    }
                }
                TextStreamPart::ReasoningDelta { text, .. } => {
                    current_reasoning.push_str(&text);
                }
                TextStreamPart::ReasoningEnd { .. } => {
                    if !current_reasoning.is_empty() {
                        outputs.push(Output::Reasoning(ReasoningOutput::new(
                            current_reasoning.clone(),
                        )));
                        current_reasoning.clear();
                    }
                }
                TextStreamPart::Source { source } => {
                    outputs.push(Output::Source(source));
                }
                TextStreamPart::File { file } => {
                    outputs.push(Output::File(GeneratedFile::from_base64(
                        &file.base64,
                        &file.media_type,
                    )));
                }
                TextStreamPart::ToolCall { tool_call } => {
                    outputs.push(Output::ToolCall(tool_call));
                }
                TextStreamPart::ToolResult { tool_result } => {
                    outputs.push(Output::ToolResult(tool_result));
                }
                TextStreamPart::ToolError { tool_error } => {
                    outputs.push(Output::ToolError(tool_error));
                }
                TextStreamPart::Error { error } => {
                    send(Event::default().event("error").data(json!({
                        "error": error
                    }).to_string()));
                    return;
                }
                _ => {}
            }
        }

        if !current_text.is_empty() {
            outputs.push(Output::Text(TextOutput::new(current_text)));
        }
        if !current_reasoning.is_empty() {
            outputs.push(Output::Reasoning(ReasoningOutput::new(current_reasoning)));
        }

        let reply_parts = outputs_to_parts(&outputs);
        let assistant_message = Message::from_parts(&session_id, "assistant", reply_parts.clone());
        if let Err(error) = Message::store(&storage, &assistant_message).await {
            send(Event::default().event("error").data(json!({
                "error": error.to_string()
            }).to_string()));
            return;
        }

        send(Event::default().event("done").data(json!({
            "context": api_context,
            "reply_parts": reply_parts
        }).to_string()));
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

fn to_api_context(ctx: SessionContext) -> ApiSessionContext {
    ApiSessionContext {
        workspace_id: ctx.workspace_id,
        session_id: ctx.session_id,
        started_at: ctx.started_at,
        ended_at: ctx.ended_at,
    }
}
