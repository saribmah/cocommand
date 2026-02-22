use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;

use crate::clipboard::{
    clear_history, get_clipboard_snapshot, list_history, record_clipboard, set_clipboard_files,
    set_clipboard_image, set_clipboard_text,
};
use crate::error::CoreError;
use crate::extension::manifest::ExtensionManifest;
use crate::extension::{Extension, ExtensionKind, ExtensionTool};

use super::manifest_tools::{merge_manifest_tools, parse_builtin_manifest};

pub struct ClipboardExtension {
    manifest: ExtensionManifest,
    tools: Vec<ExtensionTool>,
}

impl std::fmt::Debug for ClipboardExtension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClipboardExtension").finish()
    }
}

impl Default for ClipboardExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipboardExtension {
    pub fn new() -> Self {
        let manifest = parse_builtin_manifest(include_str!("clipboard_manifest.json"));

        let mut execute_map = HashMap::new();

        execute_map.insert(
            "get_clipboard",
            Arc::new(
                |_input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    crate::extension::boxed_tool_value_future("Tool result", async move {
                        let snapshot = get_clipboard_snapshot(&context.workspace).await?;
                        Ok(serde_json::to_value(snapshot).map_err(|error| {
                            CoreError::Internal(format!(
                                "failed to serialize clipboard snapshot: {error}"
                            ))
                        })?)
                    })
                },
            ) as _,
        );

        execute_map.insert(
            "set_clipboard",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    crate::extension::boxed_tool_value_future("Tool result", async move {
                        let kind = input
                            .get("kind")
                            .and_then(|value| value.as_str())
                            .ok_or_else(|| CoreError::Internal("missing kind".to_string()))?;
                        match kind {
                            "text" => {
                                let text = input
                                    .get("text")
                                    .and_then(|value| value.as_str())
                                    .ok_or_else(|| {
                                        CoreError::Internal("missing text".to_string())
                                    })?;
                                set_clipboard_text(&context.workspace, text).await?;
                                Ok(json!({ "status": "ok" }))
                            }
                            "image" => {
                                let path = input
                                    .get("imagePath")
                                    .and_then(|value| value.as_str())
                                    .ok_or_else(|| {
                                        CoreError::Internal("missing imagePath".to_string())
                                    })?;
                                let bytes = tokio::fs::read(path).await.map_err(|error| {
                                    CoreError::Internal(format!(
                                        "failed to read image {path}: {error}"
                                    ))
                                })?;
                                set_clipboard_image(&context.workspace, &bytes).await?;
                                Ok(json!({ "status": "ok" }))
                            }
                            "files" => {
                                let files = input
                                    .get("files")
                                    .and_then(|value| value.as_array())
                                    .ok_or_else(|| {
                                    CoreError::Internal("missing files".to_string())
                                })?;
                                let files = files
                                    .iter()
                                    .filter_map(|value| value.as_str().map(|item| item.to_string()))
                                    .collect::<Vec<_>>();
                                set_clipboard_files(&context.workspace, files).await?;
                                Ok(json!({ "status": "ok" }))
                            }
                            other => Err(CoreError::Internal(format!(
                                "unsupported clipboard kind: {other}"
                            ))),
                        }
                    })
                },
            ) as _,
        );

        execute_map.insert(
            "record_clipboard",
            Arc::new(
                |_input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    crate::extension::boxed_tool_value_future("Tool result", async move {
                        let entry = record_clipboard(&context.workspace).await?;
                        Ok(serde_json::to_value(entry).map_err(|error| {
                            CoreError::Internal(format!(
                                "failed to serialize clipboard entry: {error}"
                            ))
                        })?)
                    })
                },
            ) as _,
        );

        execute_map.insert(
            "list_clipboard_history",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    crate::extension::boxed_tool_value_future("Tool result", async move {
                        let limit = input.get("limit").and_then(|value| value.as_u64());
                        let items =
                            list_history(&context.workspace.storage, limit.map(|v| v as usize))
                                .await?;
                        Ok(serde_json::to_value(items).map_err(|error| {
                            CoreError::Internal(format!(
                                "failed to serialize clipboard history: {error}"
                            ))
                        })?)
                    })
                },
            ) as _,
        );

        execute_map.insert(
            "clear_clipboard_history",
            Arc::new(
                |_input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    crate::extension::boxed_tool_value_future("Tool result", async move {
                        clear_history(&context.workspace.storage).await?;
                        Ok(json!({ "status": "ok" }))
                    })
                },
            ) as _,
        );

        let tools = merge_manifest_tools(&manifest, execute_map);

        Self { manifest, tools }
    }
}

#[async_trait::async_trait]
impl Extension for ClipboardExtension {
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
