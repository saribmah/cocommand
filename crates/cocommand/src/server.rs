use axum::{routing::get, Router};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::workspace::load_or_create_workspace_config;

pub struct ServerHandle {
    addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
}

impl ServerHandle {
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
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

pub async fn start(workspace_dir: PathBuf) -> Result<ServerHandle, String> {
    load_or_create_workspace_config(&workspace_dir).map_err(|error| error.to_string())?;
    let app = Router::new().route("/health", get(health));
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

    Ok(ServerHandle {
        addr,
        shutdown: Some(shutdown_tx),
    })
}

async fn health() -> &'static str {
    "ok"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::workspace_config_path;
    use tempfile::tempdir;

    #[tokio::test]
    async fn start_creates_workspace_config() {
        let dir = tempdir().expect("tempdir");
        let mut handle = start(dir.path().to_path_buf()).await.expect("start");
        let path = workspace_config_path(dir.path());
        assert!(path.exists());
        handle.shutdown().expect("shutdown");
    }

    #[tokio::test]
    async fn start_binds_random_port() {
        let dir = tempdir().expect("tempdir");
        let mut handle = start(dir.path().to_path_buf()).await.expect("start");
        let addr = handle.addr();
        assert_ne!(addr.port(), 0);
        assert_ne!(addr.port(), 4840);
        handle.shutdown().expect("shutdown");
    }
}
