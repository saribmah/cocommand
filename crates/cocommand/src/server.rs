use axum::routing::{get, post};
use axum::{Json, Router};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{oneshot, watch};
use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi;

use crate::browser::BrowserBridge;
use crate::bus::Bus;
use crate::clipboard::spawn_clipboard_watcher;
use crate::command::runtime::SessionRuntimeRegistry;
use crate::llm::{settings_from_workspace, LlmKitProvider, LlmProvider};
use crate::oauth::OAuthManager;
use crate::platform::{default_platform, SharedPlatform};
use crate::session::SessionManager;
use crate::workspace::WorkspaceInstance;
pub mod assets;
pub mod browser;
pub mod error;
pub mod events;
pub mod extension;
pub mod invoke;
pub mod oauth;
pub mod openapi;
pub mod screenshots;
pub mod session;
pub mod system;

pub struct Server {
    addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    clipboard_shutdown: Option<watch::Sender<bool>>,
    workspace: WorkspaceInstance,
}

impl Server {
    pub async fn new(workspace_dir: PathBuf) -> Result<Self, String> {
        Self::new_with_platform(workspace_dir, default_platform()).await
    }

    pub async fn new_with_platform(
        workspace_dir: PathBuf,
        platform: SharedPlatform,
    ) -> Result<Self, String> {
        let workspace = WorkspaceInstance::new_with_platform(&workspace_dir, platform)
            .await
            .map_err(|error| error.to_string())?;
        let workspace_arc = Arc::new(workspace.clone());
        let sessions = Arc::new(SessionManager::new(workspace_arc.clone()));
        let bus = Bus::new(512);
        let settings = {
            let config = workspace.config.read().await;
            settings_from_workspace(&config.llm)
        };
        let llm: Arc<dyn LlmProvider> =
            Arc::new(LlmKitProvider::new(settings).map_err(|e| e.to_string())?);
        let oauth = OAuthManager::new(Duration::from_secs(300));
        let browser_bridge = Arc::new(BrowserBridge::new(Duration::from_secs(10)));
        let runtime_registry = Arc::new(SessionRuntimeRegistry::new(
            workspace.clone(),
            sessions.clone(),
            llm.clone(),
            bus.clone(),
        ));

        // Register extensions that need the LLM provider.
        {
            use crate::extension::builtin::agent::AgentExtension;
            use crate::extension::builtin::browser::BrowserExtension;
            use crate::extension::builtin::workspace::WorkspaceExtension;
            use crate::extension::Extension;
            let mut registry = workspace.extension_registry.write().await;
            registry.register(Arc::new(AgentExtension::new(llm.clone())) as Arc<dyn Extension>);
            registry.register(
                Arc::new(BrowserExtension::new(browser_bridge.clone())) as Arc<dyn Extension>
            );
            registry.register(Arc::new(WorkspaceExtension::new(llm.clone())) as Arc<dyn Extension>);
        }

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|error| error.to_string())?;
        let addr = listener.local_addr().map_err(|error| error.to_string())?;
        let state = Arc::new(ServerState {
            workspace,
            sessions,
            bus,
            oauth,
            browser: browser_bridge,
            addr,
            runtime_registry,
        });
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);
        let app = Router::new()
            .route("/health", get(health))
            .route("/openapi.json", get(serve_openapi))
            .route("/events", get(events::stream_events))
            .route("/session/command", get(session::session_command_history))
            .route("/sessions/command", post(session::session_command))
            .route("/sessions/context", get(session::session_context))
            .route("/workspace/extensions", get(extension::list_extensions))
            .route(
                "/workspace/extensions/open",
                post(extension::open_extension),
            )
            .route(
                "/extension/:extension_id/invoke/:tool_id",
                post(invoke::invoke_tool),
            )
            .route(
                "/extension/:extension_id/assets/*path",
                get(assets::serve_extension_asset),
            )
            .route(
                "/workspace/screenshots/:filename",
                get(screenshots::serve_screenshot),
            )
            .route(
                "/workspace/extension/system/applications",
                get(system::list_applications),
            )
            .route(
                "/workspace/extension/system/applications/open",
                post(system::open_application),
            )
            .route("/browser/ws", get(browser::ws_handler))
            .route("/browser/status", get(browser::status))
            .route("/oauth/start", post(oauth::start_flow))
            .route("/oauth/callback", get(oauth::callback))
            .route("/oauth/poll", get(oauth::poll_flow))
            .route(
                "/oauth/tokens/:ext/:provider",
                post(oauth::set_tokens)
                    .get(oauth::get_tokens)
                    .delete(oauth::delete_tokens),
            )
            .with_state(state.clone())
            .layer(cors);
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let (clipboard_shutdown_tx, clipboard_shutdown_rx) = watch::channel(false);

        tokio::spawn(async move {
            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await;
        });

        spawn_clipboard_watcher(state.workspace.clone(), clipboard_shutdown_rx, 500);

        {
            let oauth = state.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(60));
                loop {
                    interval.tick().await;
                    oauth.oauth.cleanup().await;
                }
            });
        }

        Ok(Server {
            addr,
            shutdown: Some(shutdown_tx),
            clipboard_shutdown: Some(clipboard_shutdown_tx),
            workspace: state.workspace.clone(),
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn shutdown(&mut self) -> Result<(), String> {
        if let Some(sender) = self.clipboard_shutdown.take() {
            let _ = sender.send(true);
        }
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

async fn serve_openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(openapi::ApiDoc::openapi())
}

pub(crate) struct ServerState {
    pub(crate) workspace: WorkspaceInstance,
    pub(crate) sessions: Arc<SessionManager>,
    pub(crate) bus: Bus,
    pub(crate) oauth: OAuthManager,
    pub(crate) browser: Arc<BrowserBridge>,
    pub(crate) addr: SocketAddr,
    pub(crate) runtime_registry: Arc<SessionRuntimeRegistry>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn start_creates_workspace_config() {
        let dir = tempdir().expect("tempdir");
        let mut server = Server::new(dir.path().to_path_buf()).await.expect("start");
        let workspace_id = {
            let config = server.workspace().config.read().await;
            config.workspace_id.clone()
        };
        let stored = server
            .workspace()
            .storage
            .read(&["workspace", &workspace_id])
            .await
            .expect("storage read");
        assert!(stored.is_some());
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
