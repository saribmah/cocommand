use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::Response;
use std::sync::Arc;

use crate::extension::custom::CustomExtension;
use crate::server::ServerState;

pub(crate) async fn serve_extension_asset(
    State(state): State<Arc<ServerState>>,
    Path((extension_id, asset_path)): Path<(String, String)>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let registry = state.workspace.extension_registry.read().await;
    let ext = registry
        .get(&extension_id)
        .ok_or((StatusCode::NOT_FOUND, "extension not found".to_string()))?;

    let custom = ext
        .as_any()
        .downcast_ref::<CustomExtension>()
        .ok_or((
            StatusCode::BAD_REQUEST,
            "only custom extensions support asset serving".to_string(),
        ))?;

    let base_dir = custom.extension_dir().canonicalize().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to resolve extension dir: {}", e),
        )
    })?;

    let requested = base_dir.join(&asset_path);
    let resolved = requested.canonicalize().map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            "asset not found".to_string(),
        )
    })?;

    // Path traversal guard
    if !resolved.starts_with(&base_dir) {
        return Err((StatusCode::FORBIDDEN, "path traversal denied".to_string()));
    }

    let bytes = tokio::fs::read(&resolved).await.map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            "asset not found".to_string(),
        )
    })?;

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
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}
