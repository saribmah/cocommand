use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tokio::sync::{mpsc, Mutex, Notify, RwLock};

/// Manages the WebSocket connection to the Chrome extension and
/// request/response matching for browser tool calls.
pub struct BrowserBridge {
    connection: RwLock<ConnectionState>,
    pending: Mutex<HashMap<String, PendingRequest>>,
    timeout: Duration,
}

struct ConnectionState {
    sender: Option<mpsc::UnboundedSender<String>>,
    generation: u64,
}

struct PendingRequest {
    result: Option<Result<Value, String>>,
    notify: Arc<Notify>,
}

impl BrowserBridge {
    pub fn new(timeout: Duration) -> Self {
        Self {
            connection: RwLock::new(ConnectionState {
                sender: None,
                generation: 0,
            }),
            pending: Mutex::new(HashMap::new()),
            timeout,
        }
    }

    /// Send a command to the Chrome extension and wait for a response.
    pub async fn send_command(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let message = serde_json::json!({
            "id": id,
            "method": method,
            "params": params,
        });

        let notify = Arc::new(Notify::new());

        // Insert pending request first, then send.
        {
            let mut pending = self.pending.lock().await;
            pending.insert(
                id.clone(),
                PendingRequest {
                    result: None,
                    notify: notify.clone(),
                },
            );
        }

        // Send via current connection.
        {
            let conn = self.connection.read().await;
            match &conn.sender {
                Some(sender) => {
                    let text = serde_json::to_string(&message)
                        .map_err(|e| format!("failed to serialize command: {e}"))?;
                    if sender.send(text).is_err() {
                        // Clean up pending and return error.
                        let mut pending = self.pending.lock().await;
                        pending.remove(&id);
                        return Err(
                            "Browser extension not connected. Install the Cocommand Chrome extension."
                                .to_string(),
                        );
                    }
                }
                None => {
                    let mut pending = self.pending.lock().await;
                    pending.remove(&id);
                    return Err(
                        "Browser extension not connected. Install the Cocommand Chrome extension."
                            .to_string(),
                    );
                }
            }
        }

        // Wait for response with timeout.
        let result = tokio::time::timeout(self.timeout, notify.notified()).await;

        let mut pending = self.pending.lock().await;
        if let Some(req) = pending.remove(&id) {
            if result.is_err() {
                return Err(format!("browser command '{method}' timed out"));
            }
            req.result
                .unwrap_or_else(|| Err("no response received".to_string()))
        } else {
            Err("request was cancelled".to_string())
        }
    }

    /// Called when a new Chrome extension connects via WebSocket.
    /// Returns the generation ID for this connection.
    pub async fn on_connect(&self, sender: mpsc::UnboundedSender<String>) -> u64 {
        // Fail all pending requests from the old connection.
        {
            let mut pending = self.pending.lock().await;
            for (_, req) in pending.drain() {
                req.notify.notify_one();
            }
        }

        let mut conn = self.connection.write().await;
        conn.generation += 1;
        conn.sender = Some(sender);
        let gen = conn.generation;
        tracing::info!("browser extension connected (generation {gen})");
        gen
    }

    /// Called when the Chrome extension disconnects.
    /// Only clears the sender if the generation matches (prevents
    /// stale disconnects from clobbering new connections).
    pub async fn on_disconnect(&self, generation: u64) {
        let mut conn = self.connection.write().await;
        if conn.generation == generation {
            conn.sender = None;
            tracing::info!("browser extension disconnected (generation {generation})");

            // Fail any pending requests.
            drop(conn);
            let mut pending = self.pending.lock().await;
            for (_, req) in pending.drain() {
                req.notify.notify_one();
            }
        }
    }

    /// Called when a message arrives from the Chrome extension.
    pub async fn on_message(&self, text: &str) {
        let msg: Value = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("invalid message from browser extension: {e}");
                return;
            }
        };

        let id = match msg.get("id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => {
                tracing::warn!("browser extension message missing 'id' field");
                return;
            }
        };

        let result = if let Some(error) = msg.get("error") {
            Err(error
                .as_str()
                .unwrap_or("unknown browser error")
                .to_string())
        } else {
            Ok(msg.get("result").cloned().unwrap_or(Value::Null))
        };

        let mut pending = self.pending.lock().await;
        if let Some(req) = pending.get_mut(&id) {
            req.result = Some(result);
            req.notify.notify_one();
        } else {
            tracing::warn!("received response for unknown request id: {id}");
        }
    }

    /// Whether a Chrome extension is currently connected.
    pub async fn is_connected(&self) -> bool {
        let conn = self.connection.read().await;
        conn.sender.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn not_connected_returns_error() {
        let bridge = BrowserBridge::new(Duration::from_secs(1));
        let result = bridge
            .send_command("getTabs", serde_json::json!({}))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not connected"));
    }

    #[tokio::test]
    async fn connect_and_respond() {
        let bridge = Arc::new(BrowserBridge::new(Duration::from_secs(5)));
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();

        let gen = bridge.on_connect(tx).await;
        assert!(bridge.is_connected().await);

        // Spawn a task that reads the command and sends a response.
        let bridge2 = bridge.clone();
        tokio::spawn(async move {
            if let Some(msg) = rx.recv().await {
                let cmd: Value = serde_json::from_str(&msg).unwrap();
                let id = cmd["id"].as_str().unwrap().to_string();
                let response = serde_json::json!({
                    "id": id,
                    "result": [{ "id": 1, "url": "https://example.com", "title": "Example" }]
                });
                bridge2.on_message(&response.to_string()).await;
            }
        });

        let result = bridge
            .send_command("getTabs", serde_json::json!({}))
            .await;
        assert!(result.is_ok());
        let tabs = result.unwrap();
        assert!(tabs.is_array());

        bridge.on_disconnect(gen).await;
        assert!(!bridge.is_connected().await);
    }

    #[tokio::test]
    async fn disconnect_fails_pending_requests() {
        let bridge = Arc::new(BrowserBridge::new(Duration::from_secs(5)));
        let (tx, _rx) = mpsc::unbounded_channel::<String>();

        let gen = bridge.on_connect(tx).await;

        let bridge2 = bridge.clone();
        let handle = tokio::spawn(async move {
            bridge2
                .send_command("getTabs", serde_json::json!({}))
                .await
        });

        // Give the send_command a moment to register the pending request.
        tokio::time::sleep(Duration::from_millis(50)).await;
        bridge.on_disconnect(gen).await;

        let result = handle.await.unwrap();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn timeout_returns_error() {
        let bridge = Arc::new(BrowserBridge::new(Duration::from_millis(50)));
        let (tx, _rx) = mpsc::unbounded_channel::<String>();

        bridge.on_connect(tx).await;

        let result = bridge
            .send_command("getTabs", serde_json::json!({}))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("timed out"));
    }

    #[tokio::test]
    async fn new_connection_invalidates_old() {
        let bridge = Arc::new(BrowserBridge::new(Duration::from_secs(5)));

        let (tx1, _rx1) = mpsc::unbounded_channel::<String>();
        let gen1 = bridge.on_connect(tx1).await;

        let (tx2, _rx2) = mpsc::unbounded_channel::<String>();
        let gen2 = bridge.on_connect(tx2).await;

        assert!(gen2 > gen1);
        assert!(bridge.is_connected().await);

        // Disconnecting old generation should not affect the new connection.
        bridge.on_disconnect(gen1).await;
        assert!(bridge.is_connected().await);

        bridge.on_disconnect(gen2).await;
        assert!(!bridge.is_connected().await);
    }
}
