use std::sync::Arc;

use llm_kit_core::tool::ToolSet;
use llm_kit_provider_utils::message::{AssistantMessage, Message, UserMessage};
use llm_kit_provider_utils::tool::{Tool, ToolExecutionOutput};
use serde_json::json;

use crate::application::{Application, ApplicationContext, ApplicationAction, ApplicationKind};
use crate::session::SessionManager;
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
    sessions: Arc<SessionManager>,
    session_id: &str,
    active_app_ids: &[String],
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

    for app_id in active_app_ids {
        if let Some(app) = registry.get(app_id) {
            for action in app.actions() {
                let raw_name = format!("{}.{}", app_id, action.id);
                let tool_name = sanitize_tool_name(&raw_name);
                let tool = build_tool(app.clone(), action, context.clone());
                tool_set.insert(tool_name, tool);
            }
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

fn map_kind(kind: ApplicationKind) -> &'static str {
    match kind {
        ApplicationKind::System => "system",
        ApplicationKind::BuiltIn => "built-in",
        ApplicationKind::Custom => "custom",
    }
}

fn build_search_applications_tool(workspace: Arc<WorkspaceInstance>) -> Tool {
    let execute = Arc::new(move |input: serde_json::Value, _opts| {
        let workspace = workspace.clone();
        ToolExecutionOutput::Single(Box::pin(async move {
            let query = input
                .get("query")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim()
                .to_lowercase();
            let limit = input
                .get("limit")
                .and_then(|value| value.as_u64())
                .unwrap_or(8) as usize;
            let registry = workspace
                .application_registry
                .read()
                .map_err(|_| json!({ "error": "registry lock" }))?;
            let mut items: Vec<(serde_json::Value, i64)> = registry
                .list()
                .into_iter()
                .map(|app| {
                    let id = app.id().to_string();
                    let name = app.name().to_string();
                    let kind = map_kind(app.kind()).to_string();
                    let tags = app.tags();
                    let score = match_score(&query, &name, &id, &kind);
                    (
                        json!({
                            "id": id,
                            "name": name,
                            "kind": kind,
                            "tags": tags,
                        }),
                        score,
                    )
                })
                .filter(|(_, score)| query.is_empty() || *score >= 0)
                .collect();
            items.sort_by(|a, b| b.1.cmp(&a.1));
            let results: Vec<serde_json::Value> = items
                .into_iter()
                .take(limit)
                .map(|(value, _)| value)
                .collect();
            Ok(json!({ "results": results }))
        }))
    });

    Tool::function(json!({
        "type": "object",
        "properties": {
            "query": { "type": "string" },
            "limit": { "type": "number", "minimum": 1, "maximum": 50 }
        },
        "required": ["query"]
    }))
    .with_description("Search available applications by name or id.")
    .with_execute(execute)
}

fn build_get_application_tool(workspace: Arc<WorkspaceInstance>) -> Tool {
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
                "actions": app.actions().into_iter().map(|action| {
                    json!({
                        "id": action.id,
                        "name": action.name,
                        "description": action.description,
                        "input_schema": action.input_schema,
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
    .with_description("Fetch full details for an application, including actions.")
    .with_execute(execute)
}

fn build_activate_application_tool(
    workspace: Arc<WorkspaceInstance>,
    sessions: Arc<SessionManager>,
    session_id: &str,
) -> Tool {
    let session_id = session_id.to_string();
    let execute = Arc::new(move |input: serde_json::Value, _opts| {
        let workspace = workspace.clone();
        let sessions = sessions.clone();
        let session_id = session_id.clone();
        ToolExecutionOutput::Single(Box::pin(async move {
            let app_id = input
                .get("id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| json!({ "error": "missing id" }))?
                .to_string();
            let exists = {
                let registry = workspace
                    .application_registry
                    .read()
                    .map_err(|_| json!({ "error": "registry lock" }))?;
                registry.get(&app_id).is_some()
            };
            if !exists {
                return Err(json!({ "error": "application not found" }));
            }
            sessions
                .with_session_mut(|session| {
                    let app_id = app_id.clone();
                    let session_id = session_id.clone();
                    Box::pin(async move {
                        if session.session_id != session_id {
                            return Err(crate::error::CoreError::InvalidInput(
                                "session not found".to_string(),
                            ));
                        }
                        session.open_application(&app_id);
                        Ok(())
                    })
                })
                .await
                .map_err(|error| json!({ "error": error.to_string() }))?;
            Ok(json!({ "status": "ok", "activated": true, "id": app_id }))
        }))
    });

    Tool::function(json!({
        "type": "object",
        "properties": {
            "id": { "type": "string" }
        },
        "required": ["id"]
    }))
    .with_description("Activate an application so its tools become available.")
    .with_execute(execute)
}

fn match_score(query: &str, name: &str, id: &str, kind: &str) -> i64 {
    if query.is_empty() {
        return 0;
    }
    let name_lower = name.to_lowercase();
    let id_lower = id.to_lowercase();
    let kind_lower = kind.to_lowercase();
    if name_lower.contains(query) || id_lower.contains(query) || kind_lower.contains(query) {
        return 100 + query.len() as i64;
    }
    let compact_query = query.replace(' ', "");
    let name_score = subsequence_score(&compact_query, &name_lower.replace(' ', ""));
    let id_score = subsequence_score(&compact_query, &id_lower.replace(' ', ""));
    let kind_score = subsequence_score(&compact_query, &kind_lower.replace(' ', ""));
    let best = name_score.max(id_score).max(kind_score);
    if best > 0 { best } else { -1 }
}

fn subsequence_score(query: &str, target: &str) -> i64 {
    if query.is_empty() {
        return 0;
    }
    let mut score = 0;
    let mut ti = 0;
    for ch in query.chars() {
        if let Some(found) = target[ti..].find(ch) {
            let index = ti + found;
            score += if index == ti { 2 } else { 1 };
            ti = index + 1;
        } else {
            return -1;
        }
    }
    score
}
