use crate::application::{Application, ApplicationKind, ApplicationTool};
use crate::error::CoreError;

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

#[async_trait::async_trait]
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

    fn tools(&self) -> Vec<ApplicationTool> {
        #[cfg(target_os = "macos")]
        {
            return platform_macos::installed_app_tools()
                .into_iter()
                .map(|(id, name, description, input_schema)| ApplicationTool {
                    id,
                    name,
                    description,
                    input_schema,
                })
                .collect();
        }
        #[cfg(not(target_os = "macos"))]
        {
            Vec::new()
        }
    }

    async fn execute(
        &self,
        tool_id: &str,
        input: serde_json::Value,
        _context: &crate::application::ApplicationContext,
    ) -> crate::error::CoreResult<serde_json::Value> {
        #[cfg(target_os = "macos")]
        {
            if tool_id == "open" {
                return platform_macos::open_installed_app(self.bundle_id(), &self.path)
                    .map(|_| serde_json::json!({ "status": "ok" }))
                    .map_err(CoreError::Internal);
            }
            return platform_macos::execute_installed_app_tool(tool_id, &input)
                .map_err(CoreError::Internal);
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(CoreError::Internal(
                "applescript execution only supported on macos".to_string(),
            ))
        }
    }
}
