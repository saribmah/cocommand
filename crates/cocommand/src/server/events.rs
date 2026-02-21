use axum::extract::{Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use utoipa::IntoParams;

use crate::event::CoreEvent;
use crate::server::ServerState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct EventsQuery {
    pub session_id: Option<String>,
}

#[utoipa::path(
    get,
    path = "/events",
    tag = "events",
    params(EventsQuery),
    responses(
        (status = 200, description = "SSE stream of CoreEvent payloads", content_type = "text/event-stream"),
    ),
    description = "Subscribe to real-time server events via SSE. Optionally filter by session_id."
)]
pub(crate) async fn stream_events(
    State(state): State<Arc<ServerState>>,
    Query(query): Query<EventsQuery>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.bus.subscribe();
    let session_filter = query.session_id;
    let stream = BroadcastStream::new(rx).filter_map(move |message| {
        let session_filter = session_filter.clone();
        match message {
            Ok(event) => {
                if let Some(filter) = session_filter {
                    let event_session = event_session_id(&event);
                    if event_session != filter {
                        return None;
                    }
                }
                let payload = serde_json::to_string(&event).ok()?;
                Some(Ok(Event::default().data(payload)))
            }
            Err(_) => None,
        }
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

fn event_session_id(event: &CoreEvent) -> String {
    match event {
        CoreEvent::SessionMessageStarted(payload) => payload.session_id.clone(),
        CoreEvent::SessionPartUpdated(payload) => payload.session_id.clone(),
        CoreEvent::SessionRunCompleted(payload) => payload.session_id.clone(),
        CoreEvent::SessionRunCancelled(payload) => payload.session_id.clone(),
        CoreEvent::BackgroundJobStarted(payload) => payload.session_id.clone(),
        CoreEvent::BackgroundJobCompleted(payload) => payload.session_id.clone(),
        CoreEvent::BackgroundJobFailed(payload) => payload.session_id.clone(),
        CoreEvent::SessionContextUpdated(payload) => payload.session_id.clone(),
    }
}
