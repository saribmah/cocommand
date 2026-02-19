use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::server::ServerState;

#[utoipa::path(
    get,
    path = "/events",
    tag = "events",
    responses(
        (status = 200, description = "SSE stream of CoreEvent payloads", content_type = "text/event-stream"),
    ),
    description = "Subscribe to real-time server events via SSE."
)]
pub(crate) async fn stream_events(
    State(state): State<Arc<ServerState>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.bus.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|message| match message {
        Ok(event) => {
            let payload = serde_json::to_string(&event).ok()?;
            Some(Ok(Event::default().data(payload)))
        }
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}
