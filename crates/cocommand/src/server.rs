use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::session::SessionManager;
use crate::workspace::WorkspaceInstance;
pub mod session;

pub struct Server {
    addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    workspace: WorkspaceInstance,
}

impl Server {
    pub async fn new(workspace_dir: PathBuf) -> Result<Self, String> {
        let workspace = WorkspaceInstance::load(&workspace_dir).map_err(|error| error.to_string())?;
        let workspace_arc = Arc::new(workspace.clone());
        let sessions = SessionManager::new(workspace_arc);
        let state = Arc::new(ServerState { workspace, sessions });
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);
        let app = Router::new()
            .route("/health", get(health))
            .route("/sessions/message", post(session::record_message))
            .route("/sessions/context", get(session::session_context))
            .with_state(state.clone())
            .layer(cors);
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|error| error.to_string())?;
        let addr = listener
            .local_addr()
            .map_err(|error| error.to_string())?;
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        tokio::spawn(async move {
            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await;
        });

        Ok(Server {
            addr,
            shutdown: Some(shutdown_tx),
            workspace: state.workspace.clone(),
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn shutdown(&mut self) -> Result<(), String> {
        if let Some(sender) = self.shutdown.take() {
            sender
                .send(())
                .map_err(|_| "failed to send server shutdown signal".to_string())
        } else {
            Ok(())
        }
    }

    pub fn workspace(&self) -> &WorkspaceInstance {
        &self.workspace
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

async fn health() -> &'static str {
    "ok"
}

pub(crate) struct ServerState {
    pub(crate) workspace: WorkspaceInstance,
    pub(crate) sessions: SessionManager,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::workspace_config_path;
    use tempfile::tempdir;

    #[tokio::test]
    async fn start_creates_workspace_config() {
        let dir = tempdir().expect("tempdir");
        let mut server = Server::new(dir.path().to_path_buf()).await.expect("start");
        let path = workspace_config_path(dir.path());
        assert!(path.exists());
        server.shutdown().expect("shutdown");
    }

    #[tokio::test]
    async fn start_binds_random_port() {
        let dir = tempdir().expect("tempdir");
        let mut server = Server::new(dir.path().to_path_buf()).await.expect("start");
        let addr = server.addr();
        assert_ne!(addr.port(), 0);
        assert_ne!(addr.port(), 4840);
        server.shutdown().expect("shutdown");
    }
}
