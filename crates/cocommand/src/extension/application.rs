use std::path::PathBuf;
use std::sync::Arc;

use crate::application::{Application, ApplicationContext, ApplicationKind, ApplicationTool};
use crate::error::CoreResult;
use crate::extension::host::ExtensionHost;
use crate::extension::manifest::{ExtensionManifest, ExtensionTool};
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

#[derive(Clone)]
pub struct ExtensionApplication {
    manifest: ExtensionManifest,
    tools: Vec<ApplicationTool>,
    host: Arc<ExtensionHost>,
    extension_dir: PathBuf,
    initialized: Arc<Mutex<bool>>,
}

impl ExtensionApplication {
    pub fn new(
        manifest: ExtensionManifest,
        tools: Vec<ApplicationTool>,
        host: Arc<ExtensionHost>,
        extension_dir: PathBuf,
    ) -> Self {
        Self {
            manifest,
            tools,
            host,
            extension_dir,
            initialized: Arc::new(Mutex::new(false)),
        }
    }

    pub fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }
}

#[async_trait::async_trait]
impl Application for ExtensionApplication {
    fn id(&self) -> &str {
        &self.manifest.id
    }

    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn kind(&self) -> ApplicationKind {
        ApplicationKind::Custom
    }

    fn tags(&self) -> Vec<String> {
        self.manifest
            .routing
            .as_ref()
            .and_then(|routing| routing.keywords.clone())
            .unwrap_or_default()
    }

    fn tools(&self) -> Vec<ApplicationTool> {
        self.tools.clone()
    }

    async fn initialize(&self, _context: &ApplicationContext) -> CoreResult<()> {
        let mut guard = self.initialized.lock().await;
        if *guard {
            return Ok(());
        }
        let extension_id = self.manifest.id.clone();
        let init = timeout(
            Duration::from_secs(10),
            self.host.initialize(&self.extension_dir, &extension_id),
        )
        .await
        .map_err(|_| {
            crate::error::CoreError::Internal(format!(
                "extension init timed out for {}",
                extension_id
            ))
        })??;
        if init.tools.is_empty() {
            log::warn!("extension {} initialized with no tools", extension_id);
        }
        *guard = true;
        Ok(())
    }

    async fn execute(
        &self,
        tool_id: &str,
        input: serde_json::Value,
        _context: &ApplicationContext,
    ) -> CoreResult<serde_json::Value> {
        let initialized = { *self.initialized.lock().await };
        if !initialized {
            return Err(crate::error::CoreError::InvalidInput(format!(
                "extension {} not initialized, please activate application first.",
                self.manifest.id
            )));
        }
        self.host.invoke_tool(tool_id, input).await
    }
}

pub fn tools_from_manifest(
    manifest_tools: Option<Vec<ExtensionTool>>,
    available_tools: Option<&[String]>,
) -> Vec<ApplicationTool> {
    let mut tools = Vec::new();
    if let Some(manifest_tools) = manifest_tools {
        for tool in manifest_tools {
            if let Some(available_tools) = available_tools {
                if !available_tools.contains(&tool.id) {
                    continue;
                }
            }
            tools.push(ApplicationTool {
                id: tool.id.clone(),
                name: tool.id.clone(),
                description: None,
                input_schema: tool
                    .input_schema
                    .unwrap_or_else(|| serde_json::json!({ "type": "object" })),
            });
        }
    }
    tools
}
