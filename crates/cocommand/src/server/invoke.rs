//! Generic extension tool invoke endpoint.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde_json::json;

use crate::error::CoreError;
use crate::extension::ExtensionContext;
use crate::server::error::{ApiError, ApiErrorResponse};
use crate::server::ServerState;

/// POST /extension/{extension_id}/invoke/{tool_id}
///
/// Looks up the extension and tool by ID, executes the tool with the provided
/// JSON input, and returns a standardised `{ ok, data }` / `{ ok, error }` envelope.
#[utoipa::path(
    post,
    path = "/extension/{extension_id}/invoke/{tool_id}",
    tag = "extensions",
    params(
        ("extension_id" = String, Path, description = "Extension identifier"),
        ("tool_id" = String, Path, description = "Tool identifier"),
    ),
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Tool execution result envelope with { ok, data }"),
        (status = 400, body = ApiErrorResponse),
        (status = 404, body = ApiErrorResponse),
        (status = 500, body = ApiErrorResponse),
    )
)]
pub(crate) async fn invoke_tool(
    State(state): State<Arc<ServerState>>,
    Path((extension_id, tool_id)): Path<(String, String)>,
    Json(input): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // 1. Look up extension in registry
    let extension = {
        let registry = state.workspace.extension_registry.read().await;
        registry.get(&extension_id).ok_or_else(|| {
            ApiError::not_found(format!("Extension '{extension_id}' not found"))
        })?
    };

    // 2. Find tool by ID
    let tool = extension
        .tools()
        .into_iter()
        .find(|t| t.id == tool_id)
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "Tool '{tool_id}' not found in extension '{extension_id}'"
            ))
        })?;

    // 3. Create context (session_id unused by tool execute fns invoked over HTTP)
    let context = ExtensionContext {
        workspace: Arc::new(state.workspace.clone()),
        session_id: "http-invoke".to_string(),
    };

    // 4. Activate extension if needed (starts Deno host for custom extensions)
    extension.activate(&context).await.map_err(|error| {
        ApiError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "activation_failed",
            format!("Failed to activate extension '{extension_id}': {error}"),
        )
    })?;

    // 5. Execute and return standardised response
    match (tool.execute)(input, context).await {
        Ok(data) => Ok(Json(json!({ "ok": true, "data": data }))),
        Err(error) => {
            let api_err: ApiError = match &error {
                CoreError::InvalidInput(msg) if msg.contains("not found") => {
                    ApiError::not_found(error.to_string())
                }
                CoreError::InvalidInput(_) => ApiError::bad_request(error.to_string()),
                CoreError::NotImplemented => ApiError::new(
                    axum::http::StatusCode::NOT_IMPLEMENTED,
                    "not_implemented",
                    error.to_string(),
                ),
                _ => ApiError::internal(error.to_string()),
            };
            Err(api_err)
        }
    }
}
