#[derive(Clone)]
pub struct AgentConfig {
    pub id: String,
    pub instructions: String,
    pub temperature: f64,
    pub max_output_tokens: u32,
}
