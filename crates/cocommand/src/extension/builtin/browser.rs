use std::any::Any;
use std::sync::Arc;

use serde_json::json;

use crate::browser::BrowserBridge;
use crate::error::CoreError;
use crate::extension::{boxed_tool_future, Extension, ExtensionKind, ExtensionTool};

pub struct BrowserExtension {
    bridge: Arc<BrowserBridge>,
}

impl BrowserExtension {
    pub fn new(bridge: Arc<BrowserBridge>) -> Self {
        Self { bridge }
    }
}

#[async_trait::async_trait]
impl Extension for BrowserExtension {
    fn id(&self) -> &str {
        "browser"
    }

    fn name(&self) -> &str {
        "Browser"
    }

    fn kind(&self) -> ExtensionKind {
        ExtensionKind::System
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "browser".to_string(),
            "tabs".to_string(),
            "web".to_string(),
            "system".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn tools(&self) -> Vec<ExtensionTool> {
        let bridge = self.bridge.clone();
        let get_tabs_execute = Arc::new(move |_input: serde_json::Value, _context: crate::extension::ExtensionContext| {
            let bridge = bridge.clone();
            boxed_tool_future(async move {
                let result = bridge
                    .send_command("getTabs", json!({}))
                    .await
                    .map_err(|e| CoreError::Internal(e))?;
                Ok(result)
            })
        });

        let bridge = self.bridge.clone();
        let get_active_tab_execute = Arc::new(move |_input: serde_json::Value, _context: crate::extension::ExtensionContext| {
            let bridge = bridge.clone();
            boxed_tool_future(async move {
                let result = bridge
                    .send_command("getActiveTab", json!({}))
                    .await
                    .map_err(|e| CoreError::Internal(e))?;
                Ok(result)
            })
        });

        let bridge = self.bridge.clone();
        let get_content_execute = Arc::new(move |input: serde_json::Value, _context: crate::extension::ExtensionContext| {
            let bridge = bridge.clone();
            boxed_tool_future(async move {
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
        });

        vec![
            ExtensionTool {
                id: "get_tabs".to_string(),
                name: "Get Browser Tabs".to_string(),
                description: Some(
                    "List all open browser tabs with their id, url, title, and active state"
                        .to_string(),
                ),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
                execute: get_tabs_execute,
            },
            ExtensionTool {
                id: "get_active_tab".to_string(),
                name: "Get Active Tab".to_string(),
                description: Some(
                    "Get the currently focused browser tab".to_string(),
                ),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
                execute: get_active_tab_execute,
            },
            ExtensionTool {
                id: "get_content".to_string(),
                name: "Get Page Content".to_string(),
                description: Some(
                    "Get the content of a browser tab. Optionally specify a tab ID (defaults to active tab), format (html, text, or markdown), and a CSS selector to narrow the content."
                        .to_string(),
                ),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "tabId": {
                            "type": "integer",
                            "description": "The tab ID to get content from. Defaults to the active tab."
                        },
                        "format": {
                            "type": "string",
                            "enum": ["html", "text", "markdown"],
                            "description": "The format to return the page content in. Defaults to text."
                        },
                        "cssSelector": {
                            "type": "string",
                            "description": "A CSS selector to narrow the content extraction to a specific element."
                        }
                    },
                    "additionalProperties": false
                }),
                execute: get_content_execute,
            },
        ]
    }
}
