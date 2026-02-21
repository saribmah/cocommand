use std::sync::Arc;

use cocommand_llm::LlmTool;
use serde_json::json;

use crate::tool::search_extensions::map_kind;
use crate::workspace::WorkspaceInstance;

pub fn build_get_extension_tool(workspace: Arc<WorkspaceInstance>) -> LlmTool {
    let execute = Arc::new(move |input: serde_json::Value| {
        let workspace = workspace.clone();
        Box::pin(async move {
            let app_id = input
                .get("id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| json!({ "error": "missing id" }))?;
            let registry = workspace.extension_registry.read().await;
            let app = registry
                .get(app_id)
                .ok_or_else(|| json!({ "error": "extension not found" }))?;
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
        }) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, serde_json::Value>> + Send>>
    });

    LlmTool {
        description: Some("Fetch full details for an extension, including tools.".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "id": { "type": "string" }
            },
            "required": ["id"]
        }),
        execute,
    }
}
