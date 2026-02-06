use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;

use crate::server::ServerState;
use crate::utils::time::now_secs;

#[derive(Debug, Serialize)]
pub struct OnboardingStatusResponse {
    pub completed: bool,
    pub completed_at: Option<u64>,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOnboardingRequest {
    pub completed: bool,
    pub version: Option<String>,
}

pub(crate) async fn get_onboarding_status(
    State(state): State<Arc<ServerState>>,
) -> Result<Json<OnboardingStatusResponse>, (StatusCode, String)> {
    let config = state.workspace.config.read().await;
    Ok(Json(OnboardingStatusResponse {
        completed: config.onboarding.completed,
        completed_at: config.onboarding.completed_at,
        version: config.onboarding.version.clone(),
    }))
}

pub(crate) async fn update_onboarding_status(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<UpdateOnboardingRequest>,
) -> Result<Json<OnboardingStatusResponse>, (StatusCode, String)> {
    {
        let mut config = state.workspace.config.write().await;
        config.onboarding.completed = payload.completed;
        config.onboarding.completed_at = if payload.completed {
            Some(now_secs())
        } else {
            None
        };
        if let Some(version) = payload.version {
            config.onboarding.version = version;
        }
        config.last_modified = now_secs();
    }

    persist_workspace_config(state.clone()).await?;
    let config = state.workspace.config.read().await;
    Ok(Json(OnboardingStatusResponse {
        completed: config.onboarding.completed,
        completed_at: config.onboarding.completed_at,
        version: config.onboarding.version.clone(),
    }))
}

async fn persist_workspace_config(
    state: Arc<ServerState>,
) -> Result<(), (StatusCode, String)> {
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
