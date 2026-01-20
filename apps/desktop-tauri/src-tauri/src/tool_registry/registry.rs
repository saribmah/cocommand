use std::sync::Arc;

use llm_kit_core::ToolSet;
use llm_kit_provider_utils::tool::{Tool, ToolExecutionOutput};
use serde_json::json;

use crate::applications;
use crate::storage::WorkspaceStore;
use crate::workspace::service::WorkspaceService;

pub fn build_tool_set(
    store: Arc<dyn WorkspaceStore>,
    workspace: WorkspaceService,
) -> ToolSet {
    let mut tools = ToolSet::new();

    tools.insert(
        "window.list_apps".to_string(),
        Tool::function(json!({"type": "object"}))
            .with_description("List available applications.")
            .with_execute(Arc::new(move |_input, _opts| {
                let apps = applications::all_apps();
                ToolExecutionOutput::Single(Box::pin(async move { Ok(json!(apps)) }))
            })),
    );

    let snapshot_store = store.clone();
    let snapshot_workspace = workspace.clone();
    tools.insert(
        "window.get_snapshot".to_string(),
        Tool::function(json!({"type": "object"}))
            .with_description("Get the current workspace snapshot.")
            .with_execute(Arc::new(move |_input, _opts| {
                let snapshot = snapshot_store
                    .load()
                    .map(|workspace_state| snapshot_workspace.snapshot(&workspace_state))
                    .map_err(|error| json!({ "error": error }));
                ToolExecutionOutput::Single(Box::pin(async move {
                    snapshot.map(|value| json!(value))
                }))
            })),
    );

    let open_store = store.clone();
    let open_workspace = workspace.clone();
    tools.insert(
        "window.open".to_string(),
        Tool::function(json!({
            "type": "object",
            "properties": {
                "appId": { "type": "string" }
            },
            "required": ["appId"]
        }))
        .with_description("Open an application in the workspace.")
        .with_execute(Arc::new(move |input, _opts| {
            let app_id = input.get("appId").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let result = if app_id.is_empty() {
                Err(json!({ "error": "appId is required" }))
            } else if applications::app_by_id(&app_id).is_none() {
                Err(json!({ "error": "unknown_app", "appId": app_id }))
            } else {
                match open_store.load() {
                    Ok(mut workspace_state) => {
                        open_workspace.open_app(&mut workspace_state, &app_id);
                        match open_store.save(&workspace_state) {
                            Ok(()) => Ok(json!(open_workspace.snapshot(&workspace_state))),
                            Err(error) => Err(json!({ "error": error })),
                        }
                    }
                    Err(error) => Err(json!({ "error": error })),
                }
            };
            ToolExecutionOutput::Single(Box::pin(async move { result }))
        })),
    );

    let close_store = store.clone();
    let close_workspace = workspace.clone();
    tools.insert(
        "window.close".to_string(),
        Tool::function(json!({
            "type": "object",
            "properties": {
                "appId": { "type": "string" }
            },
            "required": ["appId"]
        }))
        .with_description("Close an application in the workspace.")
        .with_execute(Arc::new(move |input, _opts| {
            let app_id = input.get("appId").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let result = if app_id.is_empty() {
                Err(json!({ "error": "appId is required" }))
            } else if applications::app_by_id(&app_id).is_none() {
                Err(json!({ "error": "unknown_app", "appId": app_id }))
            } else {
                match close_store.load() {
                    Ok(mut workspace_state) => {
                        close_workspace.close_app(&mut workspace_state, &app_id);
                        match close_store.save(&workspace_state) {
                            Ok(()) => Ok(json!(close_workspace.snapshot(&workspace_state))),
                            Err(error) => Err(json!({ "error": error })),
                        }
                    }
                    Err(error) => Err(json!({ "error": error })),
                }
            };
            ToolExecutionOutput::Single(Box::pin(async move { result }))
        })),
    );

    let focus_store = store.clone();
    let focus_workspace = workspace.clone();
    tools.insert(
        "window.focus".to_string(),
        Tool::function(json!({
            "type": "object",
            "properties": {
                "appId": { "type": "string" }
            },
            "required": ["appId"]
        }))
        .with_description("Focus an open application in the workspace.")
        .with_execute(Arc::new(move |input, _opts| {
            let app_id = input.get("appId").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let result = if app_id.is_empty() {
                Err(json!({ "error": "appId is required" }))
            } else {
                match focus_store.load() {
                    Ok(mut workspace_state) => {
                        focus_workspace.focus_app(&mut workspace_state, &app_id);
                        match focus_store.save(&workspace_state) {
                            Ok(()) => Ok(json!(focus_workspace.snapshot(&workspace_state))),
                            Err(error) => Err(json!({ "error": error })),
                        }
                    }
                    Err(error) => Err(json!({ "error": error })),
                }
            };
            ToolExecutionOutput::Single(Box::pin(async move { result }))
        })),
    );

    for app in applications::all_apps() {
        for tool in app.tools {
            let tool_id = tool.id.clone();
            let tool_name = tool.name.clone();
            let tool_desc = format!(
                "{} (Requires {} to be open.)",
                tool.description, app.id
            );
            let exec_store = store.clone();
            tools.insert(
                tool_id.clone(),
                Tool::function(json!({"type": "object"}))
                    .with_description(format!("{} - {}", tool_name, tool_desc))
                    .with_execute(Arc::new(move |_input, _opts| {
                        let app_id = tool_id.split('.').next().unwrap_or("").to_string();
                        let result = if app_id.is_empty() {
                            Err(json!({ "error": "invalid_tool_id", "toolId": tool_id }))
                        } else {
                            match exec_store.load() {
                                Ok(workspace_state) => {
                                    let app_open = workspace_state
                                        .open_apps
                                        .iter()
                                        .any(|app| app.id == app_id);
                                    if !app_open {
                                        Err(json!({ "error": "app_not_open", "appId": app_id }))
                                    } else {
                                        match applications::execute_tool(&tool_id, json!({})) {
                                            Some(result) => Ok(json!({
                                                "status": result.status,
                                                "message": result.message
                                            })),
                                            None => Err(json!({ "error": "unknown_tool", "toolId": tool_id })),
                                        }
                                    }
                                }
                                Err(error) => Err(json!({ "error": error })),
                            }
                        };
                        ToolExecutionOutput::Single(Box::pin(async move { result }))
                    })),
            );
        }
    }

    tools
}
