//! Generic extension tool invoke endpoint.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::json;

use crate::error::CoreError;
use crate::extension::ExtensionContext;
use crate::server::ServerState;

/// POST /extension/{extension_id}/invoke/{tool_id}
///
/// Looks up the extension and tool by ID, executes the tool with the provided
/// JSON input, and returns a standardised `{ ok, data }` / `{ ok, error }` envelope.
pub(crate) async fn invoke_tool(
    State(state): State<Arc<ServerState>>,
    Path((extension_id, tool_id)): Path<(String, String)>,
    Json(input): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // 1. Look up extension in registry
    let extension = {
        let registry = state.workspace.extension_registry.read().await;
        registry.get(&extension_id).ok_or_else(|| {
            error_response(
                StatusCode::NOT_FOUND,
                "not_found",
                &format!("Extension '{extension_id}' not found"),
            )
        })?
    };

    // 2. Find tool by ID
    let tool = extension
        .tools()
        .into_iter()
        .find(|t| t.id == tool_id)
        .ok_or_else(|| {
            error_response(
                StatusCode::NOT_FOUND,
                "not_found",
                &format!("Tool '{tool_id}' not found in extension '{extension_id}'"),
            )
        })?;

    // 3. Create context (session_id unused by tool execute fns invoked over HTTP)
    let context = ExtensionContext {
        workspace: Arc::new(state.workspace.clone()),
        session_id: "http-invoke".to_string(),
    };

    // 4. Execute and return standardised response
    match (tool.execute)(input, context).await {
        Ok(data) => Ok(Json(json!({ "ok": true, "data": data }))),
        Err(error) => {
            let (status, code) = match &error {
                CoreError::InvalidInput(msg) if msg.contains("not found") => {
                    (StatusCode::NOT_FOUND, "not_found")
                }
                CoreError::InvalidInput(_) => (StatusCode::BAD_REQUEST, "invalid_input"),
                CoreError::NotImplemented => (StatusCode::NOT_IMPLEMENTED, "not_implemented"),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
            };
            Err(error_response(status, code, &error.to_string()))
        }
    }
}

fn error_response(
    status: StatusCode,
    code: &str,
    message: &str,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(json!({
            "ok": false,
            "error": { "code": code, "message": message }
        })),
    )
}
