use crate::application::{Application, ApplicationKind, ApplicationTool};
use crate::error::CoreError;
use serde_json::json;

#[derive(Debug, Default)]
pub struct NoteApplication;

impl NoteApplication {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Application for NoteApplication {
    fn id(&self) -> &str {
        "notes"
    }

    fn name(&self) -> &str {
        "Notes"
    }

    fn kind(&self) -> ApplicationKind {
        ApplicationKind::BuiltIn
    }

    fn tags(&self) -> Vec<String> {
        vec!["notes".to_string(), "writing".to_string()]
    }

    fn tools(&self) -> Vec<ApplicationTool> {
        vec![
            ApplicationTool {
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
            },
            ApplicationTool {
                id: "list-notes".to_string(),
                name: "List Notes".to_string(),
                description: Some("Show recent notes".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "minimum": 1 }
                    }
                }),
            },
        ]
    }

    async fn execute(
        &self,
        tool_id: &str,
        _input: serde_json::Value,
        _context: &crate::application::ApplicationContext,
    ) -> crate::error::CoreResult<serde_json::Value> {
        Err(CoreError::Internal(format!(
            "notes tool {tool_id} not implemented"
        )))
    }
}
