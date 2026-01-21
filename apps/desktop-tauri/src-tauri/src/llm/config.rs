use super::settings::LlmSettings;

#[derive(Clone)]
pub struct LlmConfig {
    pub provider: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: String,
}

impl LlmConfig {
    /// Create a new LlmConfig from persisted LlmSettings.
    pub fn from_settings(settings: &LlmSettings) -> Self {
        Self {
            provider: settings.provider.to_string(),
            api_key: settings.effective_api_key(),
            base_url: settings.effective_base_url(),
            model: settings.effective_model(),
        }
    }
}

impl Default for LlmConfig {
    /// Create default config from environment variables (fallback when no settings exist).
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            api_key: std::env::var("COCOMMAND_LLM_API_KEY")
                .or_else(|_| std::env::var("OPENAI_API_KEY"))
                .unwrap_or_default(),
            base_url: std::env::var("COCOMMAND_LLM_BASE_URL")
                .or_else(|_| std::env::var("OPENAI_BASE_URL"))
                .ok(),
            model: std::env::var("COCOMMAND_LLM_MODEL")
                .or_else(|_| std::env::var("OPENAI_MODEL"))
                .unwrap_or_else(|_| "openai/gpt-4o-mini".to_string()),
        }
    }
}
