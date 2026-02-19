use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::Response;
use std::sync::Arc;

use crate::extension::custom::CustomExtension;
use crate::server::error::ApiError;
use crate::server::ServerState;

pub(crate) async fn serve_extension_asset(
    State(state): State<Arc<ServerState>>,
    Path((extension_id, asset_path)): Path<(String, String)>,
) -> Result<Response<Body>, ApiError> {
    let registry = state.workspace.extension_registry.read().await;
    let ext = registry
        .get(&extension_id)
        .ok_or_else(|| ApiError::not_found("extension not found"))?;

    let custom = ext
        .as_any()
        .downcast_ref::<CustomExtension>()
        .ok_or_else(|| ApiError::bad_request("only custom extensions support asset serving"))?;

    let base_dir = custom.extension_dir().canonicalize().map_err(|e| {
        ApiError::internal(format!("failed to resolve extension dir: {}", e))
    })?;

    let requested = base_dir.join(&asset_path);
    let resolved = requested
        .canonicalize()
        .map_err(|_| ApiError::not_found("asset not found"))?;

    // Path traversal guard
    if !resolved.starts_with(&base_dir) {
        return Err(ApiError::forbidden("path traversal denied"));
    }

    let bytes = tokio::fs::read(&resolved)
        .await
        .map_err(|_| ApiError::not_found("asset not found"))?;

    let content_type = match resolved
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
    {
        "js" | "mjs" => "application/javascript",
        "css" => "text/css",
        "html" => "text/html",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        _ => "application/octet-stream",
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from(bytes))
        .map_err(|e| ApiError::internal(e.to_string()))
}
