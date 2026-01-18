#[derive(Clone)]
pub struct LlmConfig {
    pub provider: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            api_key: std::env::var("COCOMMAND_LLM_API_KEY").unwrap_or_default(),
            base_url: std::env::var("COCOMMAND_LLM_BASE_URL").ok(),
            model: std::env::var("COCOMMAND_LLM_MODEL")
                .unwrap_or_else(|_| "gpt-4o-mini".to_string()),
        }
    }
}
