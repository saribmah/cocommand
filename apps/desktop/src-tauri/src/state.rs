use std::sync::{Arc, Mutex};

use cocommand::Core;
use cocommand::LlmPlanner;
use cocommand::storage::MemoryStorage;
use llm_kit_openai::OpenAIClient;
use llm_kit_provider::LanguageModel;

/// Shared application state holding the Core instance.
/// Wrapped in Arc<Mutex<_>> because Core::submit_command requires &mut self.
pub struct AppState {
    pub core: Arc<Mutex<Core>>,
}

impl AppState {
    pub fn new() -> Self {
        let storage = Box::new(MemoryStorage::new());
        let mut core = Core::new(storage);
        core.register_builtins();
        if std::env::var("OPENAI_API_KEY").is_ok() {
            let model_id =
                std::env::var("COCOMMAND_LLM_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
            let provider = OpenAIClient::new().build();
            let model: Arc<dyn LanguageModel> = Arc::new(provider.chat(model_id));
            core.set_planner(Arc::new(LlmPlanner::new(model)));
        }
        Self {
            core: Arc::new(Mutex::new(core)),
        }
    }
}
