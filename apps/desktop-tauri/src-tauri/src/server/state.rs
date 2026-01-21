use std::sync::{Arc, RwLock};

use crate::llm::client::LlmClient;
use crate::llm::config::LlmConfig;
use crate::llm::settings::{LlmSettings, LlmSettingsStore};
use crate::storage::WorkspaceStore;
use crate::workspace::service::WorkspaceService;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn WorkspaceStore>,
    pub workspace: WorkspaceService,
    /// The LLM client wrapped in RwLock for hot-reloading when settings change.
    llm: Arc<RwLock<LlmClient>>,
    /// The LLM settings store for persistence.
    llm_settings_store: Arc<LlmSettingsStore>,
}

impl AppState {
    /// Create a new AppState with the given workspace store.
    /// Loads LLM settings from disk if available, falling back to env vars.
    pub fn new(store: Arc<dyn WorkspaceStore>) -> Self {
        let settings_store = LlmSettingsStore::default_location();
        let settings = settings_store.load().unwrap_or_default();
        let config = LlmConfig::from_settings(&settings);

        AppState {
            store,
            workspace: WorkspaceService::new(),
            llm: Arc::new(RwLock::new(LlmClient::new(config))),
            llm_settings_store: Arc::new(settings_store),
        }
    }

    /// Get a clone of the current LLM client.
    pub fn llm(&self) -> LlmClient {
        self.llm.read().unwrap().clone()
    }

    /// Get a reference to the LLM settings store.
    pub fn llm_settings_store(&self) -> &LlmSettingsStore {
        &self.llm_settings_store
    }

    /// Load current LLM settings from disk.
    pub fn load_llm_settings(&self) -> Result<LlmSettings, String> {
        self.llm_settings_store.load()
    }

    /// Save LLM settings and reload the client.
    pub fn save_llm_settings(&self, settings: &LlmSettings) -> Result<(), String> {
        // Save to disk
        self.llm_settings_store.save(settings)?;

        // Rebuild the client with new settings
        let config = LlmConfig::from_settings(settings);
        let new_client = LlmClient::new(config);

        // Update the client
        let mut llm = self.llm.write().map_err(|e| e.to_string())?;
        *llm = new_client;

        Ok(())
    }

    /// Reload the LLM client from persisted settings.
    pub fn reload_llm_client(&self) -> Result<(), String> {
        let settings = self.llm_settings_store.load()?;
        let config = LlmConfig::from_settings(&settings);
        let new_client = LlmClient::new(config);

        let mut llm = self.llm.write().map_err(|e| e.to_string())?;
        *llm = new_client;

        Ok(())
    }
}
