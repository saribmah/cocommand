use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;

use crate::llm::LlmSettings;
use crate::server::ServerState;
use crate::utils::time::now_secs;
use crate::workspace::WorkspaceConfig;

#[cfg(target_os = "macos")]
use platform_macos::{
    check_accessibility, check_automation, check_screen_recording, open_permission_settings,
};

#[derive(Debug, Serialize)]
pub struct PermissionStatus {
    pub id: String,
    pub label: String,
    pub granted: bool,
    pub required: bool,
}

#[derive(Debug, Serialize)]
pub struct PermissionsResponse {
    pub platform: String,
    pub permissions: Vec<PermissionStatus>,
}

#[derive(Debug, Deserialize)]
pub struct OpenPermissionRequest {
    pub id: String,
}

pub(crate) async fn get_workspace_config(
    State(state): State<Arc<ServerState>>,
) -> Result<Json<WorkspaceConfig>, (StatusCode, String)> {
    let config = state.workspace.config.read().await;
    Ok(Json(config.clone()))
}

pub(crate) async fn update_workspace_config(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<WorkspaceConfig>,
) -> Result<Json<WorkspaceConfig>, (StatusCode, String)> {
    let updated = {
        let mut config = state.workspace.config.write().await;
        let mut next = payload;
        next.version = config.version.clone();
        next.workspace_id = config.workspace_id.clone();
        next.created_at = config.created_at;
        next.last_modified = now_secs();
        *config = next.clone();
        next
    };

    persist_workspace_config(state.clone()).await?;

    state
        .llm
        .update_settings(LlmSettings::from_workspace(&updated.llm))
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    Ok(Json(updated))
}

pub(crate) async fn get_permissions_status(
    State(_state): State<Arc<ServerState>>,
) -> Result<Json<PermissionsResponse>, (StatusCode, String)> {
    #[cfg(target_os = "macos")]
    {
        let permissions = vec![
            PermissionStatus {
                id: "accessibility".to_string(),
                label: "Accessibility".to_string(),
                granted: check_accessibility(),
                required: true,
            },
            PermissionStatus {
                id: "screen-recording".to_string(),
                label: "Screen Recording".to_string(),
                granted: check_screen_recording(),
                required: true,
            },
            PermissionStatus {
                id: "automation".to_string(),
                label: "Automation".to_string(),
                granted: check_automation().unwrap_or(false),
                required: true,
            },
        ];
        Ok(Json(PermissionsResponse {
            platform: "macos".to_string(),
            permissions,
        }))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(Json(PermissionsResponse {
            platform: "unsupported".to_string(),
            permissions: Vec::new(),
        }))
    }
}

pub(crate) async fn open_permission(
    State(_state): State<Arc<ServerState>>,
    Json(payload): Json<OpenPermissionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    #[cfg(target_os = "macos")]
    {
        open_permission_settings(&payload.id).map_err(|error| (StatusCode::BAD_REQUEST, error))?;
        Ok(Json(serde_json::json!({ "status": "ok" })))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err((StatusCode::BAD_REQUEST, "unsupported platform".to_string()))
    }
}

async fn persist_workspace_config(state: Arc<ServerState>) -> Result<(), (StatusCode, String)> {
    let value = serde_json::to_value({
        let config = state.workspace.config.read().await;
        config.clone()
    })
    .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let workspace_id = {
        let config = state.workspace.config.read().await;
        config.workspace_id.clone()
    };
    state
        .workspace
        .storage
        .write(&["workspace", &workspace_id], &value)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok(())
}
