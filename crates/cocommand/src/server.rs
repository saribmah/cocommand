use axum::{routing::get, Router};
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub async fn start() -> Result<SocketAddr, String> {
    let app = Router::new().route("/health", get(health));
    let listener = TcpListener::bind("127.0.0.1:4840")
        .await
        .map_err(|error| error.to_string())?;
    let addr = listener
        .local_addr()
        .map_err(|error| error.to_string())?;

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    Ok(addr)
}

async fn health() -> &'static str {
    "ok"
}
