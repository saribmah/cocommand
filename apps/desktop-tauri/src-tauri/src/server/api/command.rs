//! Command route handlers.
//!
//! This module handles the /command endpoint for processing user commands
//! through the agent loop.

use axum::{extract::State, Json};
use serde_json::json;

use crate::agent::{processor, registry as agent_registry};
use crate::commands::intake as command_intake;

use super::super::state::AppState;
use super::types::{CommandSubmitRequest, CommandSubmitResponse};

/// Handle the /command POST endpoint.
///
/// Processes a user command through the control→execution loop.
/// Archived workspaces are handled by the processor, which includes restore tools.
pub async fn command(
    State(state): State<AppState>,
    Json(request): Json<CommandSubmitRequest>,
) -> Json<CommandSubmitResponse> {
    if request.text.trim().is_empty() {
        return Json(CommandSubmitResponse::empty(
            "Type a command to get started.",
        ));
    }

    // Validate workspace can be loaded (but don't block on archived status)
    // The processor handles archived workspaces and provides restore functionality
    if let Err(e) = state.store.load() {
        let cmd = command_intake::normalize(command_intake::CommandRequest {
            text: request.text.clone(),
            source: Some("ui".to_string()),
        });
        return Json(CommandSubmitResponse::error(cmd, format!("Failed to load workspace: {}", e)));
    }

    // Normalize the incoming command
    let command = command_intake::normalize(command_intake::CommandRequest {
        text: request.text.clone(),
        source: Some("ui".to_string()),
    });

    // Get agent config
    let agent_config = agent_registry::default_agent();

    // Process command through the control→execution loop
    let process_result = processor::process_command(
        &state.llm,
        state.store.clone(),
        state.workspace.clone(),
        agent_config,
        &command.text,
    )
    .await;

    match process_result {
        Ok(result) => {
            let phase_str = match result.phase_used {
                processor::SessionPhase::Control => "control",
                processor::SessionPhase::Execution => "execution",
            };

            Json(CommandSubmitResponse::success(
                command,
                result.output,
                phase_str,
                result.turns_used,
            ))
        }
        Err(error) => {
            log_llm_debug(&state, &request.text).await;
            Json(CommandSubmitResponse::error(command, error))
        }
    }
}

/// Log LLM debug information when COCOMMAND_LLM_DEBUG is set.
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
