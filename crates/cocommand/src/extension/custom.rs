use std::path::PathBuf;
use std::sync::Arc;

use crate::application::{
    boxed_tool_future, Extension, ExtensionContext, ExtensionKind, ExtensionTool,
};
use crate::error::CoreResult;
use crate::extension::host::ExtensionHost;
use crate::extension::manifest::{ExtensionManifest, ExtensionTool as ManifestTool};
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

#[derive(Clone)]
pub struct CustomExtension {
    manifest: ExtensionManifest,
    tools: Vec<ExtensionTool>,
    host: Arc<ExtensionHost>,
    extension_dir: PathBuf,
    initialized: Arc<Mutex<bool>>,
}

impl CustomExtension {
    pub fn new(
        manifest: ExtensionManifest,
        host: Arc<ExtensionHost>,
        extension_dir: PathBuf,
    ) -> Self {
        let initialized = Arc::new(Mutex::new(false));
        let tools = tools_from_manifest(
            manifest.tools.clone(),
            None,
            host.clone(),
            initialized.clone(),
            &manifest.id,
        );
        Self {
            manifest,
            tools,
            host,
            extension_dir,
            initialized,
        }
    }

    pub fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }
}

#[async_trait::async_trait]
impl Extension for CustomExtension {
    fn id(&self) -> &str {
        &self.manifest.id
    }

    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn kind(&self) -> ExtensionKind {
        ExtensionKind::Custom
    }

    fn tags(&self) -> Vec<String> {
        self.manifest
            .routing
            .as_ref()
            .and_then(|routing| routing.keywords.clone())
            .unwrap_or_default()
    }

    fn tools(&self) -> Vec<ExtensionTool> {
        self.tools.clone()
    }

    async fn initialize(&self, _context: &ExtensionContext) -> CoreResult<()> {
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
}

pub fn tools_from_manifest(
    manifest_tools: Option<Vec<ManifestTool>>,
    available_tools: Option<&[String]>,
    host: Arc<ExtensionHost>,
    initialized: Arc<Mutex<bool>>,
    extension_id: &str,
) -> Vec<ExtensionTool> {
    let mut tools = Vec::new();
    if let Some(manifest_tools) = manifest_tools {
        for tool in manifest_tools {
            if let Some(available_tools) = available_tools {
                if !available_tools.contains(&tool.id) {
                    continue;
                }
            }
            let tool_id = tool.id.clone();
            let host = host.clone();
            let initialized = initialized.clone();
            let extension_id = extension_id.to_string();
            let execute = Arc::new(move |input: serde_json::Value, _context| {
                let host = host.clone();
                let initialized = initialized.clone();
                let tool_id = tool_id.clone();
                let extension_id = extension_id.clone();
                boxed_tool_future(async move {
                    let initialized = { *initialized.lock().await };
                    if !initialized {
                        return Err(crate::error::CoreError::InvalidInput(format!(
                            "extension {} not initialized, please activate extension first.",
                            extension_id
                        )));
                    }
                    host.invoke_tool(&tool_id, input).await
                })
            });
            tools.push(ExtensionTool {
                id: tool.id.clone(),
                name: tool.id.clone(),
                description: None,
                input_schema: tool
                    .input_schema
                    .unwrap_or_else(|| serde_json::json!({ "type": "object" })),
                execute,
            });
        }
    }
    tools
}
