use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;

use crate::browser::BrowserBridge;
use crate::error::CoreError;
use crate::extension::manifest::ExtensionManifest;
use crate::extension::{Extension, ExtensionKind, ExtensionTool};

use super::manifest_tools::{merge_manifest_tools, parse_builtin_manifest};

pub struct BrowserExtension {
    manifest: ExtensionManifest,
    tools: Vec<ExtensionTool>,
}

impl BrowserExtension {
    pub fn new(bridge: Arc<BrowserBridge>) -> Self {
        let manifest = parse_builtin_manifest(include_str!("browser_manifest.json"));

        let mut execute_map = HashMap::new();

        let b = bridge.clone();
        execute_map.insert(
            "get_tabs",
            Arc::new(
                move |_input: serde_json::Value, _context: crate::extension::ExtensionContext| {
                    let bridge = b.clone();
                    crate::extension::boxed_tool_value_future("Tool result", async move {
                        let result = bridge
                            .send_command("getTabs", json!({}))
                            .await
                            .map_err(|e| CoreError::Internal(e))?;
                        Ok(result)
                    })
                },
            ) as _,
        );

        let b = bridge.clone();
        execute_map.insert(
            "get_active_tab",
            Arc::new(
                move |_input: serde_json::Value, _context: crate::extension::ExtensionContext| {
                    let bridge = b.clone();
                    crate::extension::boxed_tool_value_future("Tool result", async move {
                        let result = bridge
                            .send_command("getActiveTab", json!({}))
                            .await
                            .map_err(|e| CoreError::Internal(e))?;
                        Ok(result)
                    })
                },
            ) as _,
        );

        let b = bridge.clone();
        execute_map.insert(
            "get_content",
            Arc::new(
                move |input: serde_json::Value, _context: crate::extension::ExtensionContext| {
                    let bridge = b.clone();
                    crate::extension::boxed_tool_value_future("Tool result", async move {
                        let params = json!({
                            "tabId": input.get("tabId"),
                            "format": input.get("format").and_then(|v| v.as_str()).unwrap_or("text"),
                            "cssSelector": input.get("cssSelector"),
                        });
                        let result = bridge
                            .send_command("getContent", params)
                            .await
                            .map_err(|e| CoreError::Internal(e))?;
                        Ok(result)
                    })
                },
            ) as _,
        );

        let tools = merge_manifest_tools(&manifest, execute_map);

        Self { manifest, tools }
    }
}

#[async_trait::async_trait]
impl Extension for BrowserExtension {
    fn id(&self) -> &str {
        &self.manifest.id
    }

    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn kind(&self) -> ExtensionKind {
        ExtensionKind::System
    }

    fn tags(&self) -> Vec<String> {
        self.manifest
            .routing
            .as_ref()
            .and_then(|r| r.keywords.clone())
            .unwrap_or_default()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn tools(&self) -> Vec<ExtensionTool> {
        self.tools.clone()
    }
}
