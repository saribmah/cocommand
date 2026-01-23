//! window.list_apps tool implementation.
//!
//! Lists all available applications that can be opened in the workspace.

use std::sync::Arc;

use llm_kit_provider_utils::tool::{Tool, ToolExecutionOutput};
use serde_json::json;

use crate::applications;

/// Tool ID for the list_apps tool
pub const TOOL_ID: &str = "window_list_apps";

/// Build the window.list_apps tool.
///
/// This tool lists all available applications that can be opened in the workspace.
/// It returns an array of application objects with id, name, and description.
pub fn build() -> (String, Tool) {
    let tool = Tool::function(json!({
        "type": "object",
        "properties": {},
        "required": []
    }))
    .with_description("List all available applications that can be opened in the workspace.")
    .with_execute(Arc::new(move |_input, _opts| {
        let apps: Vec<_> = applications::all_apps()
            .into_iter()
            .map(|app| {
                json!({
                    "id": app.id,
                    "name": app.name,
                    "description": app.description
                })
            })
            .collect();
        ToolExecutionOutput::Single(Box::pin(async move { Ok(json!(apps)) }))
    }));

    (TOOL_ID.to_string(), tool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let (id, _tool) = build();
        assert_eq!(id, "window_list_apps");
    }

    #[test]
    fn test_tool_has_description() {
        let (_id, tool) = build();
        // The tool has a description field set via with_description
        assert!(tool.description.is_some());
        assert!(tool
            .description
            .as_ref()
            .unwrap()
            .contains("available applications"));
    }
}
