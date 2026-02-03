use std::sync::Arc;

use llm_kit_core::tool::ToolSet;

use crate::application::{ExtensionContext, ExtensionTool};
use crate::session::SessionManager;
use crate::tool::activate_extension::build_activate_extension_tool;
use crate::tool::get_extension::build_get_extension_tool;
use crate::tool::search_extensions::build_search_extensions_tool;
use crate::workspace::WorkspaceInstance;

pub struct ToolRegistry;

impl ToolRegistry {
    pub async fn tools(
        workspace: Arc<WorkspaceInstance>,
        sessions: Arc<SessionManager>,
        session_id: &str,
        active_app_ids: &[String],
    ) -> ToolSet {
        let context = ExtensionContext {
            workspace: workspace.clone(),
            session_id: session_id.to_string(),
        };
        let registry = workspace.extension_registry.read().await;
        let mut tool_set = ToolSet::new();

        tool_set.insert(
            "search_extensions".to_string(),
            build_search_extensions_tool(workspace.clone()),
        );
        tool_set.insert(
            "get_extension".to_string(),
            build_get_extension_tool(workspace.clone()),
        );
        tool_set.insert(
            "activate_extension".to_string(),
            build_activate_extension_tool(workspace.clone(), sessions.clone(), session_id),
        );

        for app in registry.list() {
            if app.kind() != crate::application::ExtensionKind::System {
                continue;
            }
            for tool in app.tools() {
                let raw_name = format!("{}.{}", app.id(), tool.id);
                let tool_name = sanitize_tool_name(&raw_name);
                let tool = build_tool(tool, context.clone());
                tool_set.insert(tool_name, tool);
            }
        }

        for app_id in active_app_ids {
            if let Some(app) = registry.get(app_id) {
                if app.kind() == crate::application::ExtensionKind::System {
                    continue;
                }
                for tool in app.tools() {
                    let raw_name = format!("{}.{}", app_id, tool.id);
                    let tool_name = sanitize_tool_name(&raw_name);
                    let tool = build_tool(tool, context.clone());
                    tool_set.insert(tool_name, tool);
                }
            }
        }

        tool_set
    }
}

fn build_tool(
    tool: ExtensionTool,
    context: ExtensionContext,
) -> llm_kit_provider_utils::tool::Tool {
    let description = tool.description.clone();
    let schema = tool.input_schema.clone();
    let execute_context = context.clone();
    let execute_handler = tool.execute.clone();

    let execute = Arc::new(move |input: serde_json::Value, _opts| {
        let execute_context = execute_context.clone();
        let execute_handler = execute_handler.clone();
        llm_kit_provider_utils::tool::ToolExecutionOutput::Single(Box::pin(async move {
            execute_handler(input, execute_context)
                .await
                .map_err(|error| serde_json::json!({ "error": error.to_string() }))
        }))
    });

    let mut tool = llm_kit_provider_utils::tool::Tool::function(schema).with_execute(execute);
    if let Some(description) = description {
        tool = tool.with_description(description);
    }
    tool
}

fn sanitize_tool_name(name: &str) -> String {
    let mut sanitized = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }
    sanitized
}
