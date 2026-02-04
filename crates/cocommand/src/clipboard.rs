use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use tokio::time::{sleep, Duration};

use crate::error::{CoreError, CoreResult};
use crate::storage::SharedStorage;
use crate::utils::time::now_rfc3339;
use crate::workspace::WorkspaceInstance;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClipboardKind {
    Text,
    Image,
    Files,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardHistoryEntry {
    pub id: String,
    pub created_at: String,
    pub kind: ClipboardKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

const HISTORY_KEYS: [&str; 2] = ["clipboard", "history"];
const IMAGE_DIR: &str = "clipboard/images";
const CURRENT_IMAGE_DIR: &str = "clipboard/current";
const IMAGE_FORMAT: &str = "tiff";

pub async fn list_history(
    storage: &SharedStorage,
    limit: Option<usize>,
) -> CoreResult<Vec<ClipboardHistoryEntry>> {
    let mut ids = storage.list(&HISTORY_KEYS).await?;
    ids.sort();
    let mut items = Vec::new();
    for id in ids.into_iter().rev() {
        if let Some(entry) = load_history_entry(storage, &id).await? {
            items.push(entry);
            if let Some(limit) = limit {
                if items.len() >= limit {
                    break;
                }
            }
        }
    }
    Ok(items)
}

pub async fn load_history_entry(
    storage: &SharedStorage,
    id: &str,
) -> CoreResult<Option<ClipboardHistoryEntry>> {
    let value = storage.read(&[HISTORY_KEYS[0], HISTORY_KEYS[1], id]).await?;
    match value {
        Some(value) => serde_json::from_value(value).map(Some).map_err(|error| {
            CoreError::Internal(format!("failed to parse clipboard entry: {error}"))
        }),
        None => Ok(None),
    }
}

pub async fn store_history_entry(
    storage: &SharedStorage,
    entry: &ClipboardHistoryEntry,
) -> CoreResult<()> {
    let value = serde_json::to_value(entry).map_err(|error| {
        CoreError::Internal(format!("failed to serialize clipboard entry: {error}"))
    })?;
    storage
        .write(&[HISTORY_KEYS[0], HISTORY_KEYS[1], &entry.id], &value)
        .await
}

pub async fn delete_history_entry(
    storage: &SharedStorage,
    entry: &ClipboardHistoryEntry,
) -> CoreResult<()> {
    storage
        .delete(&[HISTORY_KEYS[0], HISTORY_KEYS[1], &entry.id])
        .await?;
    if let Some(path) = &entry.image_path {
        let _ = tokio::fs::remove_file(path).await;
    }
    Ok(())
}

pub async fn clear_history(storage: &SharedStorage) -> CoreResult<()> {
    let ids = storage.list(&HISTORY_KEYS).await?;
    for id in ids {
        if let Some(entry) = load_history_entry(storage, &id).await? {
            delete_history_entry(storage, &entry).await?;
        } else {
            storage
                .delete(&[HISTORY_KEYS[0], HISTORY_KEYS[1], &id])
                .await?;
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub async fn record_clipboard(
    workspace: &WorkspaceInstance,
) -> CoreResult<Option<ClipboardHistoryEntry>> {
    use platform_macos::{read_clipboard, ClipboardItem};
    let item = read_clipboard().map_err(CoreError::Internal)?;
    let Some(item) = item else {
        return Ok(None);
    };
    let id = uuid::Uuid::now_v7().to_string();
    let created_at = now_rfc3339();
    let entry = match item {
        ClipboardItem::Text(text) => ClipboardHistoryEntry {
            id,
            created_at,
            kind: ClipboardKind::Text,
            text: Some(text),
            image_path: None,
            image_format: None,
            files: None,
            source: Some("pasteboard".to_string()),
        },
        ClipboardItem::Image(bytes) => {
            let path = write_image_payload(&workspace.workspace_dir, &id, &bytes).await?;
            ClipboardHistoryEntry {
                id,
                created_at,
                kind: ClipboardKind::Image,
                text: None,
                image_path: Some(path.to_string_lossy().to_string()),
                image_format: Some(IMAGE_FORMAT.to_string()),
                files: None,
                source: Some("pasteboard".to_string()),
            }
        }
        ClipboardItem::Files(files) => ClipboardHistoryEntry {
            id,
            created_at,
            kind: ClipboardKind::Files,
            text: None,
            image_path: None,
            image_format: None,
            files: Some(files),
            source: Some("pasteboard".to_string()),
        },
    };
    store_history_entry(&workspace.storage, &entry).await?;
    prune_old_entries(workspace).await?;
    Ok(Some(entry))
}

#[cfg(not(target_os = "macos"))]
pub async fn record_clipboard(
    _workspace: &WorkspaceInstance,
) -> CoreResult<Option<ClipboardHistoryEntry>> {
    Err(CoreError::Internal(
        "clipboard tracking not supported on this platform".to_string(),
    ))
}

#[cfg(target_os = "macos")]
pub async fn get_clipboard_snapshot(
    workspace: &WorkspaceInstance,
) -> CoreResult<Option<ClipboardHistoryEntry>> {
    use platform_macos::{read_clipboard, ClipboardItem};
    let item = read_clipboard().map_err(CoreError::Internal)?;
    let Some(item) = item else {
        return Ok(None);
    };
    let id = uuid::Uuid::now_v7().to_string();
    let created_at = now_rfc3339();
    let entry = match item {
        ClipboardItem::Text(text) => ClipboardHistoryEntry {
            id,
            created_at,
            kind: ClipboardKind::Text,
            text: Some(text),
            image_path: None,
            image_format: None,
            files: None,
            source: Some("snapshot".to_string()),
        },
        ClipboardItem::Image(bytes) => {
            let path = write_current_image(&workspace.workspace_dir, &id, &bytes).await?;
            ClipboardHistoryEntry {
                id,
                created_at,
                kind: ClipboardKind::Image,
                text: None,
                image_path: Some(path.to_string_lossy().to_string()),
                image_format: Some(IMAGE_FORMAT.to_string()),
                files: None,
                source: Some("snapshot".to_string()),
            }
        }
        ClipboardItem::Files(files) => ClipboardHistoryEntry {
            id,
            created_at,
            kind: ClipboardKind::Files,
            text: None,
            image_path: None,
            image_format: None,
            files: Some(files),
            source: Some("snapshot".to_string()),
        },
    };
    Ok(Some(entry))
}

#[cfg(not(target_os = "macos"))]
pub async fn get_clipboard_snapshot(
    _workspace: &WorkspaceInstance,
) -> CoreResult<Option<ClipboardHistoryEntry>> {
    Err(CoreError::Internal(
        "clipboard not supported on this platform".to_string(),
    ))
}

#[cfg(target_os = "macos")]
pub async fn set_clipboard_text(text: &str) -> CoreResult<()> {
    use platform_macos::{write_clipboard, ClipboardItem};
    write_clipboard(ClipboardItem::Text(text.to_string())).map_err(CoreError::Internal)
}

#[cfg(not(target_os = "macos"))]
pub async fn set_clipboard_text(_text: &str) -> CoreResult<()> {
    Err(CoreError::Internal(
        "clipboard not supported on this platform".to_string(),
    ))
}

#[cfg(target_os = "macos")]
pub async fn set_clipboard_image(bytes: &[u8]) -> CoreResult<()> {
    use platform_macos::{write_clipboard, ClipboardItem};
    write_clipboard(ClipboardItem::Image(bytes.to_vec())).map_err(CoreError::Internal)
}

#[cfg(not(target_os = "macos"))]
pub async fn set_clipboard_image(_bytes: &[u8]) -> CoreResult<()> {
    Err(CoreError::Internal(
        "clipboard not supported on this platform".to_string(),
    ))
}

#[cfg(target_os = "macos")]
pub async fn set_clipboard_files(files: Vec<String>) -> CoreResult<()> {
    use platform_macos::{write_clipboard, ClipboardItem};
    write_clipboard(ClipboardItem::Files(files)).map_err(CoreError::Internal)
}

#[cfg(not(target_os = "macos"))]
pub async fn set_clipboard_files(_files: Vec<String>) -> CoreResult<()> {
    Err(CoreError::Internal(
        "clipboard not supported on this platform".to_string(),
    ))
}

#[cfg(target_os = "macos")]
pub fn spawn_clipboard_watcher(
    workspace: WorkspaceInstance,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    poll_ms: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut last_count = match platform_macos::clipboard_change_count() {
            Ok(value) => value,
            Err(_) => -1,
        };
        loop {
            if *shutdown.borrow() {
                break;
            }
            tokio::select! {
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        break;
                    }
                }
                _ = sleep(Duration::from_millis(poll_ms)) => {
                    match platform_macos::clipboard_change_count() {
                        Ok(count) if count != last_count => {
                            last_count = count;
                            let _ = record_clipboard(&workspace).await;
                        }
                        Ok(_) => {}
                        Err(_) => {}
                    }
                }
            }
        }
    })
}

#[cfg(not(target_os = "macos"))]
pub fn spawn_clipboard_watcher(
    _workspace: WorkspaceInstance,
    _shutdown: tokio::sync::watch::Receiver<bool>,
    _poll_ms: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {})
}

async fn prune_old_entries(workspace: &WorkspaceInstance) -> CoreResult<()> {
    let cutoff = chrono::Utc::now() - chrono::Duration::days(365);
    let ids = workspace.storage.list(&HISTORY_KEYS).await?;
    for id in ids {
        if let Some(entry) = load_history_entry(&workspace.storage, &id).await? {
            if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(&entry.created_at) {
                let timestamp = timestamp.with_timezone(&chrono::Utc);
                if timestamp < cutoff {
                    delete_history_entry(&workspace.storage, &entry).await?;
                }
            }
        }
    }
    Ok(())
}

async fn write_image_payload(
    workspace_dir: &Path,
    id: &str,
    bytes: &[u8],
) -> CoreResult<PathBuf> {
    write_image_to_dir(workspace_dir, IMAGE_DIR, id, bytes).await
}

async fn write_current_image(
    workspace_dir: &Path,
    id: &str,
    bytes: &[u8],
) -> CoreResult<PathBuf> {
    write_image_to_dir(workspace_dir, CURRENT_IMAGE_DIR, id, bytes).await
}

async fn write_image_to_dir(
    workspace_dir: &Path,
    dir: &str,
    id: &str,
    bytes: &[u8],
) -> CoreResult<PathBuf> {
    let mut path = workspace_dir.to_path_buf();
    path.push(dir);
    tokio::fs::create_dir_all(&path)
        .await
        .map_err(|error| {
            CoreError::Internal(format!(
                "failed to create clipboard directory {}: {error}",
                path.display()
            ))
        })?;
    path.push(format!("{id}.{IMAGE_FORMAT}"));
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|error| {
            CoreError::Internal(format!(
                "failed to write clipboard image {}: {error}",
                path.display()
            ))
        })?;
    Ok(path)
}
