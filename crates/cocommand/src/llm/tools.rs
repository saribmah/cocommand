use std::sync::Arc;

use llm_kit_core::tool::ToolSet;
use llm_kit_provider_utils::message::{AssistantMessage, Message, UserMessage};
use llm_kit_provider_utils::tool::{Tool, ToolExecutionOutput};
use serde_json::json;

use crate::application::{Application, ApplicationContext, ApplicationAction};
use crate::session::SessionMessage;
use crate::workspace::WorkspaceInstance;

pub fn session_messages_to_prompt(messages: &[SessionMessage]) -> Vec<Message> {
    messages
        .iter()
        .filter_map(|message| match message.role.as_str() {
            "user" => Some(Message::User(UserMessage::new(message.text.clone()))),
            "assistant" => Some(Message::Assistant(AssistantMessage::new(message.text.clone()))),
            _ => None,
        })
        .collect()
}

pub fn build_tool_set(
    workspace: Arc<WorkspaceInstance>,
    session_id: &str,
) -> ToolSet {
    let context = ApplicationContext {
        workspace: workspace.clone(),
        session_id: session_id.to_string(),
    };
    let registry = workspace
        .application_registry
        .read()
        .expect("failed to acquire application registry read lock");
    let mut tool_set = ToolSet::new();

    for app in registry.list() {
        let app_id = app.id().to_string();
        for action in app.actions() {
            let tool_name = format!("{}.{}", app_id, action.id);
            let tool = build_tool(app.clone(), action, context.clone());
            tool_set.insert(tool_name, tool);
        }
    }

    tool_set
}

fn build_tool(
    app: Arc<dyn Application>,
    action: ApplicationAction,
    context: ApplicationContext,
) -> Tool {
    let action_id = action.id.clone();
    let description = action.description.clone();
    let schema = action.input_schema.clone();
    let execute_context = context.clone();
    let execute_app = app.clone();

    let execute = Arc::new(move |input: serde_json::Value, _opts| {
        let execute_app = execute_app.clone();
        let execute_context = execute_context.clone();
        let action_id = action_id.clone();
        ToolExecutionOutput::Single(Box::pin(async move {
            execute_app
                .execute(&action_id, input, &execute_context)
                .await
                .map_err(|error| json!({ "error": error.to_string() }))
        }))
    });

    let mut tool = Tool::function(schema).with_execute(execute);
    if let Some(description) = description {
        tool = tool.with_description(description);
    }
    tool
}
