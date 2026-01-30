use std::sync::Arc;

use llm_kit_provider_utils::tool::{Tool, ToolExecutionOutput};
use serde_json::json;

use crate::tool::search_application::map_kind;
use crate::workspace::WorkspaceInstance;

pub fn build_get_application_tool(workspace: Arc<WorkspaceInstance>) -> Tool {
    let execute = Arc::new(move |input: serde_json::Value, _opts| {
        let workspace = workspace.clone();
        ToolExecutionOutput::Single(Box::pin(async move {
            let app_id = input
                .get("id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| json!({ "error": "missing id" }))?;
            let registry = workspace
                .application_registry
                .read()
                .map_err(|_| json!({ "error": "registry lock" }))?;
            let app = registry
                .get(app_id)
                .ok_or_else(|| json!({ "error": "application not found" }))?;
            Ok(json!({
                "id": app.id(),
                "name": app.name(),
                "kind": map_kind(app.kind()),
                "tags": app.tags(),
                "tools": app.tools().into_iter().map(|tool| {
                    json!({
                        "id": tool.id,
                        "name": tool.name,
                        "description": tool.description,
                        "input_schema": tool.input_schema,
                    })
                }).collect::<Vec<_>>()
            }))
        }))
    });

    Tool::function(json!({
        "type": "object",
        "properties": {
            "id": { "type": "string" }
        },
        "required": ["id"]
    }))
    .with_description("Fetch full details for an application, including tools.")
    .with_execute(execute)
}
