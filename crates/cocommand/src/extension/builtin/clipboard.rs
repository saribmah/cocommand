use std::sync::Arc;

use serde_json::json;

use crate::clipboard::{
    clear_history, get_clipboard_snapshot, list_history, record_clipboard, set_clipboard_files,
    set_clipboard_image, set_clipboard_text,
};
use crate::error::CoreError;
use crate::extension::{boxed_tool_future, Extension, ExtensionKind, ExtensionTool};

#[derive(Debug, Default)]
pub struct ClipboardExtension;

impl ClipboardExtension {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Extension for ClipboardExtension {
    fn id(&self) -> &str {
        "clipboard"
    }

    fn name(&self) -> &str {
        "Clipboard"
    }

    fn kind(&self) -> ExtensionKind {
        ExtensionKind::System
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "clipboard".to_string(),
            "history".to_string(),
            "system".to_string(),
        ]
    }

    fn tools(&self) -> Vec<ExtensionTool> {
        let get_execute = Arc::new(
            |_input: serde_json::Value, context: crate::extension::ExtensionContext| {
                boxed_tool_future(async move {
                    let snapshot = get_clipboard_snapshot(&context.workspace).await?;
                    Ok(serde_json::to_value(snapshot).map_err(|error| {
                        CoreError::Internal(format!(
                            "failed to serialize clipboard snapshot: {error}"
                        ))
                    })?)
                })
            },
        );
        let set_execute = Arc::new(|input: serde_json::Value, _context| {
            boxed_tool_future(async move {
                let kind = input
                    .get("kind")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| CoreError::Internal("missing kind".to_string()))?;
                match kind {
                    "text" => {
                        let text = input
                            .get("text")
                            .and_then(|value| value.as_str())
                            .ok_or_else(|| CoreError::Internal("missing text".to_string()))?;
                        set_clipboard_text(text).await?;
                        Ok(json!({ "status": "ok" }))
                    }
                    "image" => {
                        let path = input
                            .get("imagePath")
                            .and_then(|value| value.as_str())
                            .ok_or_else(|| CoreError::Internal("missing imagePath".to_string()))?;
                        let bytes = tokio::fs::read(path).await.map_err(|error| {
                            CoreError::Internal(format!("failed to read image {path}: {error}"))
                        })?;
                        set_clipboard_image(&bytes).await?;
                        Ok(json!({ "status": "ok" }))
                    }
                    "files" => {
                        let files = input
                            .get("files")
                            .and_then(|value| value.as_array())
                            .ok_or_else(|| CoreError::Internal("missing files".to_string()))?;
                        let files = files
                            .iter()
                            .filter_map(|value| value.as_str().map(|item| item.to_string()))
                            .collect::<Vec<_>>();
                        set_clipboard_files(files).await?;
                        Ok(json!({ "status": "ok" }))
                    }
                    other => Err(CoreError::Internal(format!(
                        "unsupported clipboard kind: {other}"
                    ))),
                }
            })
        });
        let record_execute = Arc::new(
            |_input: serde_json::Value, context: crate::extension::ExtensionContext| {
                boxed_tool_future(async move {
                    let entry = record_clipboard(&context.workspace).await?;
                    Ok(serde_json::to_value(entry).map_err(|error| {
                        CoreError::Internal(format!("failed to serialize clipboard entry: {error}"))
                    })?)
                })
            },
        );
        let list_execute = Arc::new(
            |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                boxed_tool_future(async move {
                    let limit = input.get("limit").and_then(|value| value.as_u64());
                    let items =
                        list_history(&context.workspace.storage, limit.map(|v| v as usize)).await?;
                    Ok(serde_json::to_value(items).map_err(|error| {
                        CoreError::Internal(format!(
                            "failed to serialize clipboard history: {error}"
                        ))
                    })?)
                })
            },
        );
        let clear_execute = Arc::new(
            |_input: serde_json::Value, context: crate::extension::ExtensionContext| {
                boxed_tool_future(async move {
                    clear_history(&context.workspace.storage).await?;
                    Ok(json!({ "status": "ok" }))
                })
            },
        );

        vec![
            ExtensionTool {
                id: "get_clipboard".to_string(),
                name: "Get Clipboard".to_string(),
                description: Some("Get the current clipboard contents".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
                execute: get_execute,
            },
            ExtensionTool {
                id: "set_clipboard".to_string(),
                name: "Set Clipboard".to_string(),
                description: Some("Set clipboard contents".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "kind": { "type": "string", "enum": ["text", "image", "files"] },
                        "text": { "type": "string" },
                        "imagePath": { "type": "string" },
                        "files": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["kind"],
                    "additionalProperties": false
                }),
                execute: set_execute,
            },
            ExtensionTool {
                id: "record_clipboard".to_string(),
                name: "Record Clipboard".to_string(),
                description: Some("Record the current clipboard contents into history".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
                execute: record_execute,
            },
            ExtensionTool {
                id: "list_clipboard_history".to_string(),
                name: "List Clipboard History".to_string(),
                description: Some("List recent clipboard history items".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "minimum": 1 }
                    },
                    "additionalProperties": false
                }),
                execute: list_execute,
            },
            ExtensionTool {
                id: "clear_clipboard_history".to_string(),
                name: "Clear Clipboard History".to_string(),
                description: Some("Clear clipboard history".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
                execute: clear_execute,
            },
        ]
    }
}
