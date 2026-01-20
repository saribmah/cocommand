//! Window route handlers.
//!
//! This module handles all /window/* endpoints for workspace management.

use axum::{extract::State, Json};

use crate::agent::{context::ContextBuilder, session::SessionPhase};
use crate::applications;

use super::super::state::AppState;
use super::types::{WindowAppRequest, WindowResponse};

/// Handle the /window/snapshot GET endpoint.
///
/// Returns the current workspace snapshot with lifecycle information.
pub async fn snapshot(State(state): State<AppState>) -> Json<WindowResponse> {
    let workspace = match state.store.load() {
        Ok(workspace) => workspace,
        Err(error) => return Json(WindowResponse::error(error)),
    };

    // Use context builder to apply lifecycle rules
    let context_builder = ContextBuilder::new(&state.workspace);
    let context = context_builder.build_readonly(&workspace, SessionPhase::Control);

    Json(WindowResponse::with_lifecycle(
        context.snapshot,
        context.lifecycle_message,
        context.is_soft_reset,
        context.is_archived,
    ))
}

/// Handle the /window/open POST endpoint.
///
/// Opens an application and mounts its tools.
pub async fn open(
    State(state): State<AppState>,
    Json(request): Json<WindowAppRequest>,
) -> Json<WindowResponse> {
    if applications::app_by_id(&request.app_id).is_none() {
        return Json(WindowResponse::error(format!(
            "Unknown app: {}",
            request.app_id
        )));
    }

    let mut workspace = match state.store.load() {
        Ok(workspace) => workspace,
        Err(error) => return Json(WindowResponse::error(error)),
    };

    // Check if workspace is archived - block open operations
    if state.workspace.is_archived(&workspace) {
        return Json(WindowResponse::archived());
    }

    state.workspace.open_app(&mut workspace, &request.app_id);
    if let Err(error) = state.store.save(&workspace) {
        return Json(WindowResponse::error(error));
    }

    let snapshot = state.workspace.snapshot(&workspace);
    Json(WindowResponse::success(snapshot))
}

/// Handle the /window/close POST endpoint.
///
/// Closes an application and unmounts its tools.
pub async fn close(
    State(state): State<AppState>,
    Json(request): Json<WindowAppRequest>,
) -> Json<WindowResponse> {
    if applications::app_by_id(&request.app_id).is_none() {
        return Json(WindowResponse::error(format!(
            "Unknown app: {}",
            request.app_id
        )));
    }

    let mut workspace = match state.store.load() {
        Ok(workspace) => workspace,
        Err(error) => return Json(WindowResponse::error(error)),
    };

    // Check if workspace is archived - block close operations
    if state.workspace.is_archived(&workspace) {
        return Json(WindowResponse::archived());
    }

    state.workspace.close_app(&mut workspace, &request.app_id);
    if let Err(error) = state.store.save(&workspace) {
        return Json(WindowResponse::error(error));
    }

    let snapshot = state.workspace.snapshot(&workspace);
    Json(WindowResponse::success(snapshot))
}

/// Handle the /window/focus POST endpoint.
///
/// Sets focus to an already-open application.
pub async fn focus(
    State(state): State<AppState>,
    Json(request): Json<WindowAppRequest>,
) -> Json<WindowResponse> {
    if applications::app_by_id(&request.app_id).is_none() {
        return Json(WindowResponse::error(format!(
            "Unknown app: {}",
            request.app_id
        )));
    }

    let mut workspace = match state.store.load() {
        Ok(workspace) => workspace,
        Err(error) => return Json(WindowResponse::error(error)),
    };

    // Check if workspace is archived - block focus operations
    if state.workspace.is_archived(&workspace) {
        return Json(WindowResponse::archived());
    }

    state.workspace.focus_app(&mut workspace, &request.app_id);
    if let Err(error) = state.store.save(&workspace) {
        return Json(WindowResponse::error(error));
    }

    let snapshot = state.workspace.snapshot(&workspace);
    Json(WindowResponse::success(snapshot))
}
