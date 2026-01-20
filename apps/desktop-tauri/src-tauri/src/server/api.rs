use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::applications;
use super::state::AppState;
use crate::agent::{registry as agent_registry, runner as agent_runner};
use crate::commands::intake as command_intake;
use crate::tool_registry::registry as tool_registry;
use crate::workspace::types::WorkspaceSnapshot;

#[derive(Deserialize)]
struct CommandSubmitRequest {
    text: String,
}

#[derive(Serialize)]
struct CommandSubmitResponse {
    status: String,
    command: Option<command_intake::CommandInput>,
    app_id: Option<String>,
    tool_id: Option<String>,
    result: Option<applications::ToolResult>,
    message: Option<String>,
}

#[derive(Deserialize)]
struct ExecuteRequest {
    tool_id: String,
    inputs: serde_json::Value,
}

#[derive(Serialize)]
struct ExecuteResponse {
    status: String,
    message: Option<String>,
}

#[derive(Deserialize)]
struct WindowAppRequest {
    #[serde(rename = "appId")]
    app_id: String,
}

#[derive(Serialize)]
struct WindowResponse {
    status: String,
    snapshot: Option<WorkspaceSnapshot>,
    message: Option<String>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/apps", get(apps))
        .route("/tools", get(tools))
        .route("/window/snapshot", get(window_snapshot))
        .route("/window/apps", get(window_apps))
        .route("/window/open", post(window_open))
        .route("/window/close", post(window_close))
        .route("/window/focus", post(window_focus))
        .route("/command", post(command))
        .route("/execute", post(execute))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn tools(State(state): State<AppState>) -> Json<Vec<applications::ToolDefinition>> {
    let workspace = state.store.load();
    if let Ok(workspace) = workspace {
        let open_ids: HashSet<String> =
            workspace.open_apps.iter().map(|app| app.id.clone()).collect();
        let tools = applications::all_apps()
            .into_iter()
            .filter(|app| open_ids.contains(&app.id))
            .flat_map(|app| app.tools)
            .collect();
        return Json(tools);
    }
    Json(Vec::new())
}

async fn apps(
    State(_state): State<AppState>,
) -> Json<Vec<applications::ApplicationDefinition>> {
    Json(applications::all_apps())
}

async fn window_apps(
    State(_state): State<AppState>,
) -> Json<Vec<applications::ApplicationDefinition>> {
    Json(applications::all_apps())
}

async fn window_snapshot(State(state): State<AppState>) -> Json<WindowResponse> {
    let workspace = match state.store.load() {
        Ok(workspace) => workspace,
        Err(error) => {
            return Json(WindowResponse {
                status: "error".to_string(),
                snapshot: None,
                message: Some(error),
            })
        }
    };

    let snapshot = state.workspace.snapshot(&workspace);
    Json(WindowResponse {
        status: "ok".to_string(),
        snapshot: Some(snapshot),
        message: None,
    })
}

async fn window_open(
    State(state): State<AppState>,
    Json(request): Json<WindowAppRequest>,
) -> Json<WindowResponse> {
    if applications::app_by_id(&request.app_id).is_none() {
        return Json(WindowResponse {
            status: "error".to_string(),
            snapshot: None,
            message: Some(format!("Unknown app: {}", request.app_id)),
        });
    }

    let mut workspace = match state.store.load() {
        Ok(workspace) => workspace,
        Err(error) => {
            return Json(WindowResponse {
                status: "error".to_string(),
                snapshot: None,
                message: Some(error),
            })
        }
    };

    state.workspace.open_app(&mut workspace, &request.app_id);
    if let Err(error) = state.store.save(&workspace) {
        return Json(WindowResponse {
            status: "error".to_string(),
            snapshot: None,
            message: Some(error),
        });
    }

    let snapshot = state.workspace.snapshot(&workspace);
    Json(WindowResponse {
        status: "ok".to_string(),
        snapshot: Some(snapshot),
        message: None,
    })
}

async fn window_close(
    State(state): State<AppState>,
    Json(request): Json<WindowAppRequest>,
) -> Json<WindowResponse> {
    if applications::app_by_id(&request.app_id).is_none() {
        return Json(WindowResponse {
            status: "error".to_string(),
            snapshot: None,
            message: Some(format!("Unknown app: {}", request.app_id)),
        });
    }

    let mut workspace = match state.store.load() {
        Ok(workspace) => workspace,
        Err(error) => {
            return Json(WindowResponse {
                status: "error".to_string(),
                snapshot: None,
                message: Some(error),
            })
        }
    };

    state.workspace.close_app(&mut workspace, &request.app_id);
    if let Err(error) = state.store.save(&workspace) {
        return Json(WindowResponse {
            status: "error".to_string(),
            snapshot: None,
            message: Some(error),
        });
    }

    let snapshot = state.workspace.snapshot(&workspace);
    Json(WindowResponse {
        status: "ok".to_string(),
        snapshot: Some(snapshot),
        message: None,
    })
}

async fn window_focus(
    State(state): State<AppState>,
    Json(request): Json<WindowAppRequest>,
) -> Json<WindowResponse> {
    if applications::app_by_id(&request.app_id).is_none() {
        return Json(WindowResponse {
            status: "error".to_string(),
            snapshot: None,
            message: Some(format!("Unknown app: {}", request.app_id)),
        });
    }

    let mut workspace = match state.store.load() {
        Ok(workspace) => workspace,
        Err(error) => {
            return Json(WindowResponse {
                status: "error".to_string(),
                snapshot: None,
                message: Some(error),
            })
        }
    };

    state.workspace.focus_app(&mut workspace, &request.app_id);
    if let Err(error) = state.store.save(&workspace) {
        return Json(WindowResponse {
            status: "error".to_string(),
            snapshot: None,
            message: Some(error),
        });
    }

    let snapshot = state.workspace.snapshot(&workspace);
    Json(WindowResponse {
        status: "ok".to_string(),
        snapshot: Some(snapshot),
        message: None,
    })
}

async fn command(
    State(state): State<AppState>,
    Json(request): Json<CommandSubmitRequest>,
) -> Json<CommandSubmitResponse> {
    if request.text.trim().is_empty() {
        return Json(CommandSubmitResponse {
            status: "empty".to_string(),
            command: None,
            app_id: None,
            tool_id: None,
            result: None,
            message: Some("Type a command to get started.".to_string()),
        });
    }

    let workspace = match state.store.load() {
        Ok(workspace) => workspace,
        Err(error) => {
            return Json(CommandSubmitResponse {
                status: "error".to_string(),
                command: None,
                app_id: None,
                tool_id: None,
                result: None,
                message: Some(error),
            })
        }
    };

    let command = command_intake::normalize(command_intake::CommandRequest {
        text: request.text,
        source: Some("ui".to_string()),
    });

    let snapshot = state.workspace.snapshot(&workspace);
    let tools = tool_registry::build_tool_set(state.store.clone(), state.workspace.clone());
    let agent = agent_registry::default_agent();
    let prompt = agent_registry::build_prompt(&command.text, &snapshot);
    let result = match agent_runner::run_command(&state.llm, &agent, tools, prompt).await {
        Ok(result) => result,
        Err(error) => {
            log_llm_debug(&state, &command.text).await;
            return Json(CommandSubmitResponse {
                status: "error".to_string(),
                command: Some(command),
                app_id: None,
                tool_id: None,
                result: None,
                message: Some(error),
            })
        }
    };

    Json(CommandSubmitResponse {
        status: "ok".to_string(),
        command: Some(command),
        app_id: None,
        tool_id: None,
        result: Some(applications::ToolResult {
            status: "ok".to_string(),
            message: result,
        }),
        message: None,
    })
}

async fn execute(
    State(state): State<AppState>,
    Json(request): Json<ExecuteRequest>,
) -> Json<ExecuteResponse> {
    let workspace = match state.store.load() {
        Ok(workspace) => workspace,
        Err(error) => {
            return Json(ExecuteResponse {
                status: "error".to_string(),
                message: Some(error),
            })
        }
    };

    let app_id = request.tool_id.split('.').next().unwrap_or("");
    if app_id.is_empty() {
        return Json(ExecuteResponse {
            status: "error".to_string(),
            message: Some(format!("Invalid tool id: {}", request.tool_id)),
        });
    }
    let app_open = workspace.open_apps.iter().any(|app| app.id == app_id);
    if !app_open {
        return Json(ExecuteResponse {
            status: "error".to_string(),
            message: Some(format!("App not open: {}", app_id)),
        });
    }

    let tool_allowed = applications::app_by_id(app_id)
        .map(|app| app.tools.iter().any(|tool| tool.id == request.tool_id))
        .unwrap_or(false);
    if !tool_allowed {
        return Json(ExecuteResponse {
            status: "error".to_string(),
            message: Some(format!("Unknown tool: {}", request.tool_id)),
        });
    }

    match applications::execute_tool(&request.tool_id, request.inputs) {
        Some(result) => Json(ExecuteResponse {
            status: result.status,
            message: Some(result.message),
        }),
        None => Json(ExecuteResponse {
            status: "error".to_string(),
            message: Some(format!("Unknown tool: {}", request.tool_id)),
        }),
    }
}

async fn log_llm_debug(state: &AppState, command: &str) {
    if std::env::var("COCOMMAND_LLM_DEBUG").is_err() {
        return;
    }

    let config = state.llm.config();
    let api_key = if !config.api_key.is_empty() {
        config.api_key.clone()
    } else {
        std::env::var("OPENAI_API_KEY").unwrap_or_default()
    };
    if api_key.is_empty() {
        eprintln!("LLM debug: missing API key");
        return;
    }

    let base_url = config
        .base_url
        .clone()
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    let base_url = base_url.trim_end_matches('/').to_string();
    let url = format!("{}/chat/completions", base_url);
    let payload = json!({
        "model": config.model,
        "messages": [
            { "role": "user", "content": command }
        ]
    });

    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            match resp.text().await {
                Ok(body) => {
                    eprintln!("LLM debug status: {}", status);
                    eprintln!("LLM debug response: {}", body);
                }
                Err(error) => {
                    eprintln!("LLM debug failed reading response: {}", error);
                }
            }
        }
        Err(error) => {
            eprintln!("LLM debug request failed: {}", error);
        }
    }
}
