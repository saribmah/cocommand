use std::sync::Arc;

use crate::application::{Application, ApplicationContext, ApplicationKind, ApplicationTool};
use crate::error::CoreResult;
use crate::extension::host::ExtensionHost;
use crate::extension::manifest::{ExtensionManifest, ExtensionTool};

#[derive(Clone)]
pub struct ExtensionApplication {
    manifest: ExtensionManifest,
    tools: Vec<ApplicationTool>,
    host: Arc<ExtensionHost>,
}

impl ExtensionApplication {
    pub fn new(
        manifest: ExtensionManifest,
        tools: Vec<ApplicationTool>,
        host: Arc<ExtensionHost>,
    ) -> Self {
        Self {
            manifest,
            tools,
            host,
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

    async fn execute(
        &self,
        tool_id: &str,
        input: serde_json::Value,
        _context: &ApplicationContext,
    ) -> CoreResult<serde_json::Value> {
        self.host.invoke_tool(tool_id, input).await
    }
}

pub fn tools_from_manifest(
    manifest_tools: Option<Vec<ExtensionTool>>,
    available_tools: &[String],
) -> Vec<ApplicationTool> {
    let mut tools = Vec::new();
    if let Some(manifest_tools) = manifest_tools {
        for tool in manifest_tools {
            if !available_tools.contains(&tool.id) {
                continue;
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
    } else {
        for id in available_tools {
            tools.push(ApplicationTool {
                id: id.clone(),
                name: id.clone(),
                description: None,
                input_schema: serde_json::json!({ "type": "object" }),
            });
        }
    }
    tools
}
