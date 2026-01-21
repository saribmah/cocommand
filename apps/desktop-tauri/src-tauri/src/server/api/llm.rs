//! LLM settings route handlers.
//!
//! This module handles endpoints for managing LLM provider settings,
//! including API key configuration.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::llm::settings::{AuthMethod, LlmProvider, RedactedLlmSettings};

use super::super::state::AppState;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Response for GET /llm/settings
#[derive(Serialize)]
pub struct LlmSettingsResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<RedactedLlmSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl LlmSettingsResponse {
    pub fn success(settings: RedactedLlmSettings) -> Self {
        Self {
            status: "ok".to_string(),
            settings: Some(settings),
            message: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            settings: None,
            message: Some(message.into()),
        }
    }
}

/// Request for POST /llm/settings
#[derive(Deserialize)]
pub struct UpdateLlmSettingsRequest {
    pub provider: Option<String>,
    pub auth_method: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

/// Response for POST /llm/settings
#[derive(Serialize)]
pub struct UpdateLlmSettingsResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<RedactedLlmSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl UpdateLlmSettingsResponse {
    pub fn success(settings: RedactedLlmSettings) -> Self {
        Self {
            status: "ok".to_string(),
            settings: Some(settings),
            message: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            settings: None,
            message: Some(message.into()),
        }
    }
}

/// Response for listing available providers
#[derive(Serialize)]
pub struct ProvidersResponse {
    pub status: String,
    pub providers: Vec<ProviderInfo>,
}

#[derive(Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub supports_api_key: bool,
    pub default_base_url: Option<String>,
    pub default_models: Vec<String>,
}

// ============================================================================
// Route Handlers
// ============================================================================

/// GET /llm/settings - Get current LLM settings (redacted).
pub async fn get_settings(State(state): State<AppState>) -> Json<LlmSettingsResponse> {
    match state.load_llm_settings() {
        Ok(settings) => Json(LlmSettingsResponse::success(settings.redacted())),
        Err(e) => Json(LlmSettingsResponse::error(format!("Failed to load settings: {}", e))),
    }
}

/// POST /llm/settings - Update LLM settings.
pub async fn update_settings(
    State(state): State<AppState>,
    Json(request): Json<UpdateLlmSettingsRequest>,
) -> Json<UpdateLlmSettingsResponse> {
    // Load current settings to merge with updates
    let mut settings = state.load_llm_settings().unwrap_or_default();

    // Update provider if specified
    if let Some(provider) = &request.provider {
        settings.provider = match provider.to_lowercase().as_str() {
            "openai" => LlmProvider::OpenAI,
            "anthropic" => LlmProvider::Anthropic,
            "custom" => LlmProvider::Custom,
            _ => {
                return Json(UpdateLlmSettingsResponse::error(format!(
                    "Unknown provider: {}. Valid options: openai, anthropic, custom",
                    provider
                )));
            }
        };
    }

    // Update auth method if specified (only api_key is supported)
    if let Some(auth_method) = &request.auth_method {
        settings.auth_method = match auth_method.to_lowercase().as_str() {
            "api_key" | "apikey" => AuthMethod::ApiKey,
            _ => {
                return Json(UpdateLlmSettingsResponse::error(format!(
                    "Unknown auth method: {}. Valid options: api_key",
                    auth_method
                )));
            }
        };
    }

    // Update API key if specified
    if let Some(api_key) = &request.api_key {
        settings.api_key = if api_key.is_empty() {
            None
        } else {
            Some(api_key.clone())
        };
    }

    // Update base URL if specified
    if let Some(base_url) = &request.base_url {
        settings.base_url = if base_url.is_empty() {
            None
        } else {
            Some(base_url.clone())
        };
    }

    // Update model if specified
    if let Some(model) = &request.model {
        if !model.is_empty() {
            settings.model = model.clone();
        }
    }

    // Save settings and reload the LLM client
    match state.save_llm_settings(&settings) {
        Ok(()) => Json(UpdateLlmSettingsResponse::success(settings.redacted())),
        Err(e) => Json(UpdateLlmSettingsResponse::error(format!(
            "Failed to save settings: {}",
            e
        ))),
    }
}

/// GET /llm/providers - List available providers.
pub async fn list_providers() -> Json<ProvidersResponse> {
    let providers = vec![
        ProviderInfo {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            supports_api_key: true,
            default_base_url: Some("https://api.openai.com/v1".to_string()),
            default_models: vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-3.5-turbo".to_string(),
            ],
        },
        ProviderInfo {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            supports_api_key: true,
            default_base_url: Some("https://api.anthropic.com/v1".to_string()),
            default_models: vec![
                "claude-sonnet-4-20250514".to_string(),
                "claude-3-5-sonnet-20241022".to_string(),
                "claude-3-opus-20240229".to_string(),
                "claude-3-haiku-20240307".to_string(),
            ],
        },
        ProviderInfo {
            id: "custom".to_string(),
            name: "Custom (OpenAI-compatible)".to_string(),
            supports_api_key: true,
            default_base_url: None,
            default_models: vec![],
        },
    ];

    Json(ProvidersResponse {
        status: "ok".to_string(),
        providers,
    })
}
