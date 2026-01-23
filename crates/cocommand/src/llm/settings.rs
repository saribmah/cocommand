//! LLM Settings persistence module.
//!
//! This module handles user-configurable LLM settings that persist across
//! application restarts. Settings are stored in the app data directory.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Supported LLM providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    OpenAI,
    Anthropic,
    Custom,
}

impl Default for LlmProvider {
    fn default() -> Self {
        LlmProvider::OpenAI
    }
}

impl std::fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmProvider::OpenAI => write!(f, "openai"),
            LlmProvider::Anthropic => write!(f, "anthropic"),
            LlmProvider::Custom => write!(f, "custom"),
        }
    }
}

/// Authentication method for the LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    ApiKey,
}

impl Default for AuthMethod {
    fn default() -> Self {
        AuthMethod::ApiKey
    }
}

/// Persisted LLM settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmSettings {
    /// Selected LLM provider.
    pub provider: LlmProvider,
    /// Authentication method (API key only for now).
    pub auth_method: AuthMethod,
    /// API key (when using API key auth).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Base URL for the LLM API (required for custom providers).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Model identifier to use. Empty string means use env fallback.
    #[serde(default)]
    pub model: String,
}

impl Default for LlmSettings {
    fn default() -> Self {
        Self {
            provider: LlmProvider::default(),
            auth_method: AuthMethod::default(),
            api_key: None,
            base_url: None,
            model: String::new(), // Empty to allow env var fallback
        }
    }
}

impl LlmSettings {
    /// Get the effective API key for the current settings.
    /// Returns the stored API key or falls back to env vars.
    pub fn effective_api_key(&self) -> String {
        // First, check stored API key
        if let Some(key) = &self.api_key {
            if !key.is_empty() {
                return key.clone();
            }
        }

        // Fall back to environment variables based on provider
        match self.provider {
            LlmProvider::Anthropic => std::env::var("COCOMMAND_LLM_API_KEY")
                .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
                .unwrap_or_default(),
            LlmProvider::OpenAI | LlmProvider::Custom => std::env::var("COCOMMAND_LLM_API_KEY")
                .or_else(|_| std::env::var("OPENAI_API_KEY"))
                .unwrap_or_default(),
        }
    }

    /// Get the effective base URL for the current settings.
    pub fn effective_base_url(&self) -> Option<String> {
        // Use stored base URL first
        if let Some(url) = &self.base_url {
            if !url.is_empty() {
                return Some(url.clone());
            }
        }

        // Provider-specific default URLs from env or hardcoded
        match self.provider {
            LlmProvider::OpenAI => std::env::var("COCOMMAND_LLM_BASE_URL")
                .or_else(|_| std::env::var("OPENAI_BASE_URL"))
                .ok(),
            LlmProvider::Anthropic => std::env::var("COCOMMAND_LLM_BASE_URL")
                .or_else(|_| std::env::var("ANTHROPIC_BASE_URL"))
                .ok()
                .or_else(|| Some("https://api.anthropic.com/v1".to_string())),
            LlmProvider::Custom => std::env::var("COCOMMAND_LLM_BASE_URL")
                .or_else(|_| std::env::var("OPENAI_BASE_URL"))
                .ok(),
        }
    }

    /// Get the effective model identifier.
    pub fn effective_model(&self) -> String {
        // Use stored model if non-empty
        if !self.model.is_empty() {
            return self.model.clone();
        }

        // Fall back to environment variables
        if let Ok(model) = std::env::var("COCOMMAND_LLM_MODEL") {
            return model;
        }

        // Provider-specific fallback
        match self.provider {
            LlmProvider::OpenAI => std::env::var("OPENAI_MODEL")
                .unwrap_or_else(|_| "gpt-4o-mini".to_string()),
            LlmProvider::Anthropic => std::env::var("ANTHROPIC_MODEL")
                .unwrap_or_else(|_| "claude-3-5-sonnet-20241022".to_string()),
            LlmProvider::Custom => std::env::var("OPENAI_MODEL")
                .unwrap_or_else(|_| "gpt-4o-mini".to_string()),
        }
    }

    /// Check if the settings have valid credentials configured.
    pub fn has_valid_credentials(&self) -> bool {
        !self.effective_api_key().is_empty()
    }

    /// Create a redacted copy of settings safe for API responses.
    pub fn redacted(&self) -> RedactedLlmSettings {
        RedactedLlmSettings {
            provider: self.provider.clone(),
            auth_method: self.auth_method.clone(),
            has_api_key: self.api_key.as_ref().map(|k| !k.is_empty()).unwrap_or(false),
            base_url: self.base_url.clone(),
            model: self.effective_model(),
        }
    }
}

/// Redacted LLM settings safe for API responses (no secrets).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactedLlmSettings {
    pub provider: LlmProvider,
    pub auth_method: AuthMethod,
    pub has_api_key: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    pub model: String,
}

/// LLM settings store that persists to disk.
pub struct LlmSettingsStore {
    path: PathBuf,
}

impl LlmSettingsStore {
    /// Create a new settings store at the given path.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Create a settings store in the default location.
    /// Uses COCO_LLM_SETTINGS_PATH env var or falls back to app data directory.
    pub fn default_location() -> Self {
        let path = std::env::var("COCO_LLM_SETTINGS_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                // Use platform-appropriate app data directory
                dirs::data_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("cocommand")
                    .join("llm_settings.json")
            });
        Self::new(path)
    }

    /// Load settings from disk, returning defaults if not found.
    pub fn load(&self) -> Result<LlmSettings, String> {
        if !self.path.exists() {
            return Ok(LlmSettings::default());
        }
        let data = fs::read_to_string(&self.path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    }

    /// Save settings to disk.
    pub fn save(&self, settings: &LlmSettings) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let data = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
        fs::write(&self.path, data).map_err(|e| e.to_string())
    }

    /// Get the path to the settings file.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = LlmSettings::default();
        assert_eq!(settings.provider, LlmProvider::OpenAI);
        assert_eq!(settings.auth_method, AuthMethod::ApiKey);
        assert!(settings.model.is_empty()); // Empty to allow env fallback
    }

    #[test]
    fn test_redacted_settings() {
        let mut settings = LlmSettings::default();
        settings.api_key = Some("secret_key".to_string());

        let redacted = settings.redacted();
        assert!(redacted.has_api_key);
    }

    #[test]
    fn test_serialization() {
        let settings = LlmSettings {
            provider: LlmProvider::Anthropic,
            auth_method: AuthMethod::ApiKey,
            api_key: Some("sk-test".to_string()),
            base_url: Some("https://custom.api.com".to_string()),
            model: "claude-3-opus".to_string(),
        };

        let json = serde_json::to_string(&settings).unwrap();
        let parsed: LlmSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.provider, LlmProvider::Anthropic);
        assert_eq!(parsed.model, "claude-3-opus");
    }

    #[test]
    fn test_env_fallback_for_model() {
        let settings = LlmSettings::default();
        // With empty model, should fall back to env or default
        let model = settings.effective_model();
        // Should get some model (either from env or default)
        assert!(!model.is_empty());
    }
}
