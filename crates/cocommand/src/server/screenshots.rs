use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::Response;
use std::sync::Arc;

use crate::server::ServerState;

pub(crate) async fn serve_screenshot(
    State(state): State<Arc<ServerState>>,
    Path(filename): Path<String>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let screenshots_dir = state.workspace.workspace_dir.join("screenshots");

    let base = screenshots_dir.canonicalize().map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            "screenshots directory not found".to_string(),
        )
    })?;

    let requested = screenshots_dir.join(&filename);
    let resolved = requested.canonicalize().map_err(|_| {
        (StatusCode::NOT_FOUND, "screenshot not found".to_string())
    })?;

    // Path traversal guard
    if !resolved.starts_with(&base) {
        return Err((StatusCode::FORBIDDEN, "path traversal denied".to_string()));
    }

    let bytes = tokio::fs::read(&resolved).await.map_err(|_| {
        (StatusCode::NOT_FOUND, "screenshot not found".to_string())
    })?;

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
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}
