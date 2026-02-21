#[derive(Debug, Clone)]
pub struct LlmSettings {
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub system_prompt: String,
    pub temperature: f64,
    pub max_output_tokens: u32,
    pub max_steps: usize,
}

impl Default for LlmSettings {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: None,
            model: "gpt-4o-mini".to_string(),
            system_prompt: "You are Cocommand, a helpful command assistant.".to_string(),
            temperature: 0.7,
            max_output_tokens: 80000,
            max_steps: 8,
        }
    }
}
