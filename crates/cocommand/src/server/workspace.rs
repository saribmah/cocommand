use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;

use crate::extension::{Extension, ExtensionContext, ExtensionKind, ExtensionTool};
use crate::server::ServerState;
use crate::utils::time::now_secs;
use crate::workspace::{WorkspaceAiPreferences, WorkspaceTheme};
use crate::llm::LlmSettings;

#[cfg(target_os = "macos")]
use platform_macos::{
    check_accessibility, check_automation, check_screen_recording, open_permission_settings,
};

#[derive(Debug, Serialize)]
pub struct ApplicationInfo {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub tags: Vec<String>,
    pub tools: Vec<ApplicationToolInfo>,
}

#[derive(Debug, Serialize)]
pub struct ApplicationToolInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct OpenApplicationRequest {
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct AiSettingsResponse {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub system_prompt: String,
    pub temperature: f64,
    pub max_output_tokens: u32,
    pub max_steps: usize,
    pub has_api_key: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAiSettingsRequest {
    pub provider: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub temperature: Option<f64>,
    pub max_output_tokens: Option<u32>,
    pub max_steps: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceSettingsResponse {
    pub name: String,
    pub theme: WorkspaceTheme,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkspaceSettingsRequest {
    pub name: Option<String>,
    pub theme_mode: Option<String>,
    pub theme_accent: Option<String>,
}

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

pub(crate) async fn list_applications(
    State(state): State<Arc<ServerState>>,
) -> Json<Vec<ApplicationInfo>> {
    let registry = state
        .workspace
        .extension_registry
        .read()
        .await;
    let apps = registry
        .list()
        .into_iter()
        .map(|app| map_application(app.as_ref()))
        .collect();
    Json(apps)
}

pub(crate) async fn open_application(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<OpenApplicationRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let app_id = payload.id;
    let app = {
        let registry = state
            .workspace
            .extension_registry
            .read()
            .await;
        registry
            .get(&app_id)
            .ok_or((StatusCode::NOT_FOUND, "application not found".to_string()))?
    };

    let supports_open = app.tools().into_iter().any(|tool| tool.id == "open");
    if !supports_open {
        return Err((StatusCode::BAD_REQUEST, "application cannot be opened".to_string()));
    }

    let session_id = state
        .sessions
        .with_session_mut(|session| {
            let app_id = app_id.clone();
            Box::pin(async move {
                session.activate_extension(&app_id);
                let context = session.context(None).await?;
                Ok(context.session_id)
            })
        })
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    let context = ExtensionContext {
        workspace: Arc::new(state.workspace.clone()),
        session_id,
    };
    let tool = app
        .tools()
        .into_iter()
        .find(|tool| tool.id == "open")
        .ok_or((StatusCode::BAD_REQUEST, "application cannot be opened".to_string()))?;
    let output = (tool.execute)(serde_json::json!({}), context)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    Ok(Json(output))
}

pub(crate) async fn get_ai_settings(
    State(state): State<Arc<ServerState>>,
) -> Result<Json<AiSettingsResponse>, (StatusCode, String)> {
    let config = state.workspace.config.read().await;
    let ai = &config.ai;
    Ok(Json(AiSettingsResponse {
        provider: ai.provider.clone(),
        base_url: ai.base_url.clone(),
        model: ai.model.clone(),
        system_prompt: ai.system_prompt.clone(),
        temperature: ai.temperature,
        max_output_tokens: ai.max_output_tokens,
        max_steps: ai.max_steps,
        has_api_key: ai
            .api_key
            .as_ref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false),
    }))
}

pub(crate) async fn get_workspace_settings(
    State(state): State<Arc<ServerState>>,
) -> Result<Json<WorkspaceSettingsResponse>, (StatusCode, String)> {
    let config = state.workspace.config.read().await;
    Ok(Json(WorkspaceSettingsResponse {
        name: config.name.clone(),
        theme: config.theme.clone(),
    }))
}

pub(crate) async fn update_workspace_settings(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<UpdateWorkspaceSettingsRequest>,
) -> Result<Json<WorkspaceSettingsResponse>, (StatusCode, String)> {
    {
        let mut config = state.workspace.config.write().await;
        if let Some(name) = payload.name {
            config.name = name;
        }
        if let Some(mode) = payload.theme_mode {
            config.theme.mode = mode;
        }
        if let Some(accent) = payload.theme_accent {
            config.theme.accent = accent;
        }
        config.last_modified = now_secs();
    }

    persist_workspace_config(state.clone()).await?;
    let config = state.workspace.config.read().await;
    Ok(Json(WorkspaceSettingsResponse {
        name: config.name.clone(),
        theme: config.theme.clone(),
    }))
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
        open_permission_settings(&payload.id)
            .map_err(|error| (StatusCode::BAD_REQUEST, error))?;
        Ok(Json(serde_json::json!({ "status": "ok" })))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err((StatusCode::BAD_REQUEST, "unsupported platform".to_string()))
    }
}

pub(crate) async fn update_ai_settings(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<UpdateAiSettingsRequest>,
) -> Result<Json<AiSettingsResponse>, (StatusCode, String)> {
    let updated = {
        let mut config = state.workspace.config.write().await;
        apply_ai_update(&mut config.ai, payload);
        config.ai.clone()
    };

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

    state
        .llm
        .update_settings(LlmSettings::from_workspace(&updated))
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    Ok(Json(AiSettingsResponse {
        provider: updated.provider,
        base_url: updated.base_url,
        model: updated.model,
        system_prompt: updated.system_prompt,
        temperature: updated.temperature,
        max_output_tokens: updated.max_output_tokens,
        max_steps: updated.max_steps,
        has_api_key: updated
            .api_key
            .as_ref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false),
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

fn map_application(app: &dyn Extension) -> ApplicationInfo {
    ApplicationInfo {
        id: app.id().to_string(),
        name: app.name().to_string(),
        kind: map_kind(app.kind()),
        tags: app.tags(),
        tools: app
            .tools()
            .into_iter()
            .map(map_tool)
            .collect(),
    }
}

fn map_tool(tool: ExtensionTool) -> ApplicationToolInfo {
    ApplicationToolInfo {
        id: tool.id,
        name: tool.name,
        description: tool.description,
        input_schema: tool.input_schema,
    }
}

fn map_kind(kind: ExtensionKind) -> String {
    match kind {
        ExtensionKind::System => "system",
        ExtensionKind::BuiltIn => "built-in",
        ExtensionKind::Custom => "custom",
    }
    .to_string()
}

fn apply_ai_update(ai: &mut WorkspaceAiPreferences, payload: UpdateAiSettingsRequest) {
    if let Some(provider) = payload.provider {
        ai.provider = provider;
    }
    if let Some(base_url) = payload.base_url {
        ai.base_url = base_url;
    }
    if let Some(api_key) = payload.api_key {
        if api_key.trim().is_empty() {
            ai.api_key = None;
        } else {
            ai.api_key = Some(api_key);
        }
    }
    if let Some(model) = payload.model {
        ai.model = model;
    }
    if let Some(system_prompt) = payload.system_prompt {
        ai.system_prompt = system_prompt;
    }
    if let Some(temperature) = payload.temperature {
        ai.temperature = temperature;
    }
    if let Some(max_output_tokens) = payload.max_output_tokens {
        ai.max_output_tokens = max_output_tokens;
    }
    if let Some(max_steps) = payload.max_steps {
        ai.max_steps = max_steps;
    }
}
