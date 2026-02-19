use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::Response;
use std::sync::Arc;

use crate::server::error::ApiError;
use crate::server::ServerState;

pub(crate) async fn serve_screenshot(
    State(state): State<Arc<ServerState>>,
    Path(filename): Path<String>,
) -> Result<Response<Body>, ApiError> {
    let screenshots_dir = state.workspace.workspace_dir.join("screenshots");

    let base = screenshots_dir
        .canonicalize()
        .map_err(|_| ApiError::not_found("screenshots directory not found"))?;

    let requested = screenshots_dir.join(&filename);
    let resolved = requested
        .canonicalize()
        .map_err(|_| ApiError::not_found("screenshot not found"))?;

    // Path traversal guard
    if !resolved.starts_with(&base) {
        return Err(ApiError::forbidden("path traversal denied"));
    }

    let bytes = tokio::fs::read(&resolved)
        .await
        .map_err(|_| ApiError::not_found("screenshot not found"))?;

    let content_type = match resolved
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "tiff" | "tif" => "image/tiff",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from(bytes))
        .map_err(|e| ApiError::internal(e.to_string()))
}
