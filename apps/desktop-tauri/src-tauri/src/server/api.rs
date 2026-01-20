//! API routes for the cocommand server.
//!
//! This module provides the HTTP API layer organized into submodules:
//! - `apps`: Application and tool listing/execution endpoints
//! - `command`: Command processing endpoint
//! - `window`: Window/workspace management endpoints
//! - `types`: Shared request/response types
//!
//! # Route Structure
//!
//! ```text
//! GET  /health         - Health check
//! GET  /apps           - List all applications
//! GET  /tools          - List tools for open apps
//! GET  /window/snapshot - Get workspace snapshot
//! GET  /window/apps    - List all applications (alias)
//! POST /window/open    - Open an application
//! POST /window/close   - Close an application
//! POST /window/focus   - Focus an application
//! POST /command        - Process a user command
//! POST /execute        - Execute a tool directly
//! ```

pub mod apps;
pub mod command;
pub mod types;
pub mod window;

use axum::{
    routing::{get, post},
    Router,
};

use super::state::AppState;

/// Build the API router with all routes.
pub fn router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(apps::health))
        // Application routes
        .route("/apps", get(apps::list_apps))
        .route("/tools", get(apps::list_tools))
        .route("/execute", post(apps::execute))
        // Window routes
        .route("/window/snapshot", get(window::snapshot))
        .route("/window/apps", get(apps::window_apps))
        .route("/window/open", post(window::open))
        .route("/window/close", post(window::close))
        .route("/window/focus", post(window::focus))
        // Command route
        .route("/command", post(command::command))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::state::AppState;
    use crate::storage::memory::MemoryStore;
    use std::sync::Arc;

    #[test]
    fn test_router_builds() {
        let store = Arc::new(MemoryStore::default());
        let state = AppState::new(store);
        let _router = router(state);
        // If we get here, the router built successfully
    }
}
