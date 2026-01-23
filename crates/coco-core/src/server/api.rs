//! API routes for the cocommand server.
//!
//! This module provides the HTTP API layer organized into submodules:
//! - `apps`: Application and tool listing/execution endpoints
//! - `command`: Command processing endpoint
//! - `llm`: LLM settings endpoints
//! - `window`: Window/workspace management endpoints
//! - `types`: Shared request/response types
//!
//! # Route Structure
//!
//! ```text
//! GET  /health              - Health check
//! GET  /apps                - List all applications
//! GET  /tools               - List tools for open apps
//! GET  /window/snapshot     - Get workspace snapshot
//! GET  /window/apps         - List all applications (alias)
//! POST /window/open         - Open an application
//! POST /window/close        - Close an application
//! POST /window/focus        - Focus an application
//! POST /window/restore      - Restore an archived workspace
//! POST /command             - Process a user command
//! POST /execute             - Execute a tool directly
//! GET  /llm/settings        - Get current LLM settings
//! POST /llm/settings        - Update LLM settings
//! GET  /llm/providers       - List available LLM providers
//! ```

pub mod apps;
pub mod command;
pub mod llm;
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
        .route("/window/restore", post(window::restore))
        // Command route
        .route("/command", post(command::command))
        // LLM settings routes
        .route("/llm/settings", get(llm::get_settings))
        .route("/llm/settings", post(llm::update_settings))
        .route("/llm/providers", get(llm::list_providers))
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
