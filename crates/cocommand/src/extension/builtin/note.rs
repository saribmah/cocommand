use std::sync::Arc;

use crate::application::{boxed_tool_future, Extension, ExtensionKind, ExtensionTool};
use crate::error::CoreError;
use serde_json::json;

#[derive(Debug, Default)]
pub struct NoteExtension;

impl NoteExtension {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Extension for NoteExtension {
    fn id(&self) -> &str {
        "notes"
    }

    fn name(&self) -> &str {
        "Notes"
    }

    fn kind(&self) -> ExtensionKind {
        ExtensionKind::BuiltIn
    }

    fn tags(&self) -> Vec<String> {
        vec!["notes".to_string(), "writing".to_string()]
    }

    fn tools(&self) -> Vec<ExtensionTool> {
        let create_execute = Arc::new(|_input: serde_json::Value, _context| {
            boxed_tool_future(async move {
                Err(CoreError::Internal(
                    "notes tool create-note not implemented".to_string(),
                ))
            })
        });
        let list_execute = Arc::new(|_input: serde_json::Value, _context| {
            boxed_tool_future(async move {
                Err(CoreError::Internal(
                    "notes tool list-notes not implemented".to_string(),
                ))
            })
        });

        vec![
            ExtensionTool {
                id: "create-note".to_string(),
                name: "Create Note".to_string(),
                description: Some("Create a new note".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "title": { "type": "string" },
                        "content": { "type": "string" }
                    },
                    "required": ["content"],
                }),
                execute: create_execute,
            },
            ExtensionTool {
                id: "list-notes".to_string(),
                name: "List Notes".to_string(),
                description: Some("Show recent notes".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "minimum": 1 }
                    }
                }),
                execute: list_execute,
            },
        ]
    }
}
