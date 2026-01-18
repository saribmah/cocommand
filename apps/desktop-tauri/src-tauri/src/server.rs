pub mod api;
pub mod state;

use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

pub async fn start() -> Result<SocketAddr, String> {
    let state = state::AppState::default();
    let app: Router = api::router(state).layer(CorsLayer::permissive());
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
