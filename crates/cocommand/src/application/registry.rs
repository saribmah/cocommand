use std::collections::HashMap;
use std::sync::Arc;

use crate::application::Application;

#[derive(Default)]
pub struct ApplicationRegistry {
    applications: HashMap<String, Arc<dyn Application>>,
}

impl ApplicationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, app: Arc<dyn Application>) -> Option<Arc<dyn Application>> {
        self.applications.insert(app.id().to_string(), app)
    }

    pub fn get(&self, app_id: &str) -> Option<Arc<dyn Application>> {
        self.applications.get(app_id).cloned()
    }

    pub fn list(&self) -> Vec<Arc<dyn Application>> {
        self.applications.values().cloned().collect()
    }
}
