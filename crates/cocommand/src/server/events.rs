use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use serde_json::json;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::server::ServerState;

pub(crate) async fn stream_events(
    State(state): State<Arc<ServerState>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.bus.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|message| match message {
        Ok(event) => {
            let event_type = std::any::type_name_of_val(event.as_ref());
            let payload = json!({ "type": event_type });
            Some(Ok(Event::default().data(payload.to_string())))
        }
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}
