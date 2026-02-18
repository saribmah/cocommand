use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::Json;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::server::ServerState;

#[derive(Serialize)]
pub struct BrowserStatus {
    pub connected: bool,
}

pub(crate) async fn ws_handler(
    State(state): State<Arc<ServerState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(state, socket))
}

async fn handle_socket(state: Arc<ServerState>, socket: WebSocket) {
    let (mut ws_sink, mut ws_stream) = socket.split();

    // Create a channel for outgoing messages from BrowserBridge -> WS.
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let generation = state.browser.on_connect(tx).await;

    // Write task: forward messages from the bridge to the WebSocket.
    let write_task = async move {
        while let Some(msg) = rx.recv().await {
            if ws_sink.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    };

    // Read task: forward incoming WebSocket messages to the bridge.
    let bridge = state.browser.clone();
    let read_task = async move {
        while let Some(Ok(msg)) = ws_stream.next().await {
            match msg {
                Message::Text(text) => {
                    bridge.on_message(&text).await;
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    };

    // Run both tasks, clean up when either finishes.
    tokio::select! {
        _ = write_task => {},
        _ = read_task => {},
    }

    state.browser.on_disconnect(generation).await;
}

pub(crate) async fn status(
    State(state): State<Arc<ServerState>>,
) -> Json<BrowserStatus> {
    Json(BrowserStatus {
        connected: state.browser.is_connected().await,
    })
}
