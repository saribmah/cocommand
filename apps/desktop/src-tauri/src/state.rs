use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cocommand::Core;
use cocommand::LlmPlanner;
use cocommand::storage::MemoryStorage;
use llm_kit_openai_compatible::OpenAICompatibleClient;
use llm_kit_provider::LanguageModel;
use tauri::Manager;

/// Shared application state holding the Core instance.
/// Wrapped in Arc<Mutex<_>> because Core::submit_command requires &mut self.
pub struct AppState {
    pub core: Arc<Mutex<Core>>,
    pub workspace_dir: PathBuf,
}

impl AppState {
    pub fn new(workspace_dir: PathBuf) -> Result<Self, String> {
        let storage = Box::new(MemoryStorage::new());
        let mut core = Core::new_with_workspace_dir(storage, &workspace_dir)
            .map_err(|e| e.to_string())?;
        core.register_builtins();
        if std::env::var("COCOMMAND_LLM_API_KEY").is_ok() {
            let api_key =
                std::env::var("COCOMMAND_LLM_API_KEY").unwrap_or_else(|_| "".to_string());
            let model_id =
                std::env::var("COCOMMAND_LLM_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
            let base_url =
                std::env::var("COCOMMAND_LLM_BASE_URL").unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());
            let provider = OpenAICompatibleClient::new()
                .base_url(base_url)
                .api_key(api_key)
                .build();
            let model: Arc<dyn LanguageModel> = provider.model(model_id);
            core.set_planner_with_label(Arc::new(LlmPlanner::new(model)), "llm");
        }
        Ok(Self {
            core: Arc::new(Mutex::new(core)),
            workspace_dir,
        })
    }
}

pub fn resolve_workspace_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let base_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let workspace_dir = base_dir.join("workspace");
    std::fs::create_dir_all(&workspace_dir).map_err(|error| error.to_string())?;
    Ok(workspace_dir)
}
