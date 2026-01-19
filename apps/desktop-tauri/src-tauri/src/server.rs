pub mod api;
pub mod state;

use axum::Router;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use crate::storage;
use crate::storage::WorkspaceStore;

pub async fn start() -> Result<SocketAddr, String> {
    let store: Arc<dyn WorkspaceStore> = match std::env::var("COCO_WORKSPACE_PATH") {
        Ok(path) => Arc::new(storage::file::FileStore::new(PathBuf::from(path))),
        Err(_) => Arc::new(storage::memory::MemoryStore::default()),
    };
    let state = state::AppState::new(store);
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
