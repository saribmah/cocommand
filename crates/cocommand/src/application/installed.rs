use crate::application::{Application, ApplicationAction, ApplicationKind};

#[derive(Debug, Clone)]
pub struct InstalledApplication {
    id: String,
    name: String,
    bundle_id: Option<String>,
    path: String,
}

impl InstalledApplication {
    pub fn new(id: String, name: String, bundle_id: Option<String>, path: String) -> Self {
        Self {
            id,
            name,
            bundle_id,
            path,
        }
    }

    pub fn bundle_id(&self) -> Option<&str> {
        self.bundle_id.as_deref()
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

impl Application for InstalledApplication {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> ApplicationKind {
        ApplicationKind::System
    }

    fn tags(&self) -> Vec<String> {
        Vec::new()
    }

    fn actions(&self) -> Vec<ApplicationAction> {
        Vec::new()
    }
}
