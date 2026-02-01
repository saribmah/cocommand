use std::sync::Arc;

use llm_kit_core::tool::ToolSet;

use crate::application::{ApplicationContext, ApplicationTool};
use crate::session::SessionManager;
use crate::tool::activate_application::build_activate_application_tool;
use crate::tool::get_application::build_get_application_tool;
use crate::tool::search_application::build_search_applications_tool;
use crate::workspace::WorkspaceInstance;

pub struct ToolRegistry;

impl ToolRegistry {
    pub async fn tools(
        workspace: Arc<WorkspaceInstance>,
        sessions: Arc<SessionManager>,
        session_id: &str,
        active_app_ids: &[String],
    ) -> ToolSet {
        let context = ApplicationContext {
            workspace: workspace.clone(),
            session_id: session_id.to_string(),
        };
        let registry = workspace.application_registry.read().await;
        let mut tool_set = ToolSet::new();

        tool_set.insert(
            "search_applications".to_string(),
            build_search_applications_tool(workspace.clone()),
        );
        tool_set.insert(
            "get_application".to_string(),
            build_get_application_tool(workspace.clone()),
        );
        tool_set.insert(
            "activate_application".to_string(),
            build_activate_application_tool(workspace.clone(), sessions.clone(), session_id),
        );

        if let Some(system_app) = registry.get("system") {
            for tool in system_app.tools() {
                let raw_name = format!("{}.{}", system_app.id(), tool.id);
                let tool_name = sanitize_tool_name(&raw_name);
                let tool = build_tool(tool, context.clone());
                tool_set.insert(tool_name, tool);
            }
        }

        for app_id in active_app_ids {
            if let Some(app) = registry.get(app_id) {
                if app.id() == "system" {
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
    tool: ApplicationTool,
    context: ApplicationContext,
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
