//! Filesystem API endpoints.

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::extension::builtin::filesystem::FileSystemExtension;
use crate::server::ServerState;

use filesystem::FileSystemIndexManager;

/// Request payload for index status.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStatusRequest {
    /// Root path to check status for. Defaults to workspace watch_root if not provided.
    pub root: Option<String>,
    /// Paths to ignore. Defaults to workspace ignore_paths if not provided.
    pub ignore_paths: Option<Vec<String>>,
}

/// Response payload for index status.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStatusResponse {
    pub state: String,
    pub root: String,
    pub ignored_paths: Vec<String>,
    pub indexed_entries: usize,
    pub scanned_files: usize,
    pub scanned_dirs: usize,
    pub started_at: Option<u64>,
    pub last_update_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub errors: usize,
    pub watcher_enabled: bool,
    pub cache_path: String,
    pub rescan_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

impl From<filesystem::IndexStatus> for IndexStatusResponse {
    fn from(status: filesystem::IndexStatus) -> Self {
        Self {
            state: status.state,
            root: status.root,
            ignored_paths: status.ignored_paths,
            indexed_entries: status.indexed_entries,
            scanned_files: status.scanned_files,
            scanned_dirs: status.scanned_dirs,
            started_at: status.started_at,
            last_update_at: status.last_update_at,
            finished_at: status.finished_at,
            errors: status.errors,
            watcher_enabled: status.watcher_enabled,
            cache_path: status.cache_path,
            rescan_count: status.rescan_count,
            last_error: status.last_error,
        }
    }
}

/// POST /extension/filesystem/status
///
/// Returns the current filesystem index status for the workspace.
pub(crate) async fn index_status(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<IndexStatusRequest>,
) -> Result<Json<IndexStatusResponse>, (StatusCode, String)> {
    let (watch_root, default_ignore_paths) = {
        let config = state.workspace.config.read().await;
        let prefs = &config.preferences.filesystem;
        (prefs.watch_root.clone(), prefs.ignore_paths.clone())
    };

    let root_raw = payload.root.unwrap_or_else(|| {
        if watch_root.trim().is_empty() {
            "~".to_string()
        } else {
            watch_root
        }
    });

    let workspace_dir = state.workspace.workspace_dir.clone();
    let root = normalize_path(&root_raw, &workspace_dir).map_err(|e| {
        (StatusCode::BAD_REQUEST, e.to_string())
    })?;

    let ignore_paths_raw = payload.ignore_paths.unwrap_or(default_ignore_paths);
    let ignore_paths = normalize_ignore_paths(&ignore_paths_raw, &root).map_err(|e| {
        (StatusCode::BAD_REQUEST, e.to_string())
    })?;

    let index_cache_dir = workspace_dir.join("storage/filesystem-indexes");

    // Get the index manager from the filesystem extension via downcasting
    let manager = get_filesystem_index_manager(&state).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let result = tokio::task::spawn_blocking(move || {
        manager
            .index_status(root, index_cache_dir, ignore_paths)
            .map_err(CoreError::from)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("task failed: {e}")))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(result.into()))
}

fn normalize_path(raw: &str, workspace_dir: &std::path::Path) -> Result<PathBuf, CoreError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CoreError::InvalidInput("path must not be empty".to_string()));
    }

    let candidate = if trimmed == "~" || trimmed.starts_with("~/") || trimmed.starts_with("~\\") {
        expand_home(trimmed)?
    } else {
        PathBuf::from(trimmed)
    };

    if candidate.is_absolute() {
        Ok(candidate)
    } else {
        Ok(workspace_dir.join(candidate))
    }
}

fn normalize_ignore_paths(raw_paths: &[String], root: &std::path::Path) -> Result<Vec<PathBuf>, CoreError> {
    let mut normalized = Vec::new();
    for raw_path in raw_paths {
        if raw_path.trim().is_empty() {
            continue;
        }
        let candidate = if raw_path == "~" || raw_path.starts_with("~/") || raw_path.starts_with("~\\") {
            expand_home(raw_path)?
        } else {
            let path = PathBuf::from(raw_path);
            if path.is_absolute() {
                path
            } else {
                root.join(path)
            }
        };
        normalized.push(std::fs::canonicalize(&candidate).unwrap_or(candidate));
    }
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn expand_home(raw: &str) -> Result<PathBuf, CoreError> {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| CoreError::Internal("HOME is not set".to_string()))?;
    if raw == "~" {
        return Ok(home);
    }
    let rest = raw
        .strip_prefix("~/")
        .or_else(|| raw.strip_prefix("~\\"))
        .unwrap_or_default();
    Ok(home.join(rest))
}

/// Gets the FileSystemIndexManager from the filesystem extension via downcasting.
async fn get_filesystem_index_manager(
    state: &ServerState,
) -> Result<Arc<FileSystemIndexManager>, CoreError> {
    let registry = state.workspace.extension_registry.read().await;
    let extension = registry.get("filesystem").ok_or_else(|| {
        CoreError::Internal("filesystem extension not found".to_string())
    })?;

    let fs_extension = extension
        .as_any()
        .downcast_ref::<FileSystemExtension>()
        .ok_or_else(|| {
            CoreError::Internal("failed to downcast to FileSystemExtension".to_string())
        })?;

    Ok(fs_extension.index_manager().clone())
}
