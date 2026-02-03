use std::collections::HashMap;
use std::sync::Arc;

use crate::extension::Extension;

#[derive(Default)]
pub struct ExtensionRegistry {
    extensions: HashMap<String, Arc<dyn Extension>>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, app: Arc<dyn Extension>) -> Option<Arc<dyn Extension>> {
        self.extensions.insert(app.id().to_string(), app)
    }

    pub fn get(&self, app_id: &str) -> Option<Arc<dyn Extension>> {
        self.extensions.get(app_id).cloned()
    }

    pub fn list(&self) -> Vec<Arc<dyn Extension>> {
        self.extensions.values().cloned().collect()
    }
}
