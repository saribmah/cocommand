use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::time::{sleep, Duration};

use crate::error::{CoreError, CoreResult};
use crate::platform::ClipboardItem as PlatformClipboardItem;
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
    let value = storage
        .read(&[HISTORY_KEYS[0], HISTORY_KEYS[1], id])
        .await?;
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

pub async fn record_clipboard(
    workspace: &WorkspaceInstance,
) -> CoreResult<Option<ClipboardHistoryEntry>> {
    let item = workspace
        .platform
        .clipboard_read()
        .map_err(|error| match error {
            CoreError::Internal(message)
                if message == "clipboard not supported on this platform" =>
            {
                CoreError::Internal("clipboard tracking not supported on this platform".to_string())
            }
            other => other,
        })?;
    let Some(item) = item else {
        return Ok(None);
    };

    let id = uuid::Uuid::now_v7().to_string();
    let created_at = now_rfc3339();
    let entry = match item {
        PlatformClipboardItem::Text(text) => ClipboardHistoryEntry {
            id,
            created_at,
            kind: ClipboardKind::Text,
            text: Some(text),
            image_path: None,
            image_format: None,
            files: None,
            source: Some("pasteboard".to_string()),
        },
        PlatformClipboardItem::Image(bytes) => {
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
        PlatformClipboardItem::Files(files) => ClipboardHistoryEntry {
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

pub async fn get_clipboard_snapshot(
    workspace: &WorkspaceInstance,
) -> CoreResult<Option<ClipboardHistoryEntry>> {
    let item = workspace.platform.clipboard_read()?;
    let Some(item) = item else {
        return Ok(None);
    };

    let id = uuid::Uuid::now_v7().to_string();
    let created_at = now_rfc3339();
    let entry = match item {
        PlatformClipboardItem::Text(text) => ClipboardHistoryEntry {
            id,
            created_at,
            kind: ClipboardKind::Text,
            text: Some(text),
            image_path: None,
            image_format: None,
            files: None,
            source: Some("snapshot".to_string()),
        },
        PlatformClipboardItem::Image(bytes) => {
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
        PlatformClipboardItem::Files(files) => ClipboardHistoryEntry {
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

pub async fn set_clipboard_text(workspace: &WorkspaceInstance, text: &str) -> CoreResult<()> {
    workspace
        .platform
        .clipboard_write(PlatformClipboardItem::Text(text.to_string()))
}

pub async fn set_clipboard_image(workspace: &WorkspaceInstance, bytes: &[u8]) -> CoreResult<()> {
    workspace
        .platform
        .clipboard_write(PlatformClipboardItem::Image(bytes.to_vec()))
}

pub async fn set_clipboard_files(
    workspace: &WorkspaceInstance,
    files: Vec<String>,
) -> CoreResult<()> {
    workspace
        .platform
        .clipboard_write(PlatformClipboardItem::Files(files))
}

pub fn spawn_clipboard_watcher(
    workspace: WorkspaceInstance,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    poll_ms: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut last_count = workspace.platform.clipboard_change_count().ok();
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
                    match workspace.platform.clipboard_change_count() {
                        Ok(count) if last_count != Some(count) => {
                            last_count = Some(count);
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

async fn write_image_payload(workspace_dir: &Path, id: &str, bytes: &[u8]) -> CoreResult<PathBuf> {
    write_image_to_dir(workspace_dir, IMAGE_DIR, id, bytes).await
}

async fn write_current_image(workspace_dir: &Path, id: &str, bytes: &[u8]) -> CoreResult<PathBuf> {
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
    tokio::fs::create_dir_all(&path).await.map_err(|error| {
        CoreError::Internal(format!(
            "failed to create clipboard directory {}: {error}",
            path.display()
        ))
    })?;
    path.push(format!("{id}.{IMAGE_FORMAT}"));
    tokio::fs::write(&path, bytes).await.map_err(|error| {
        CoreError::Internal(format!(
            "failed to write clipboard image {}: {error}",
            path.display()
        ))
    })?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tempfile::tempdir;

    use crate::platform::Platform;
    use crate::workspace::WorkspaceInstance;

    use super::{record_clipboard, set_clipboard_text, ClipboardKind, PlatformClipboardItem};

    #[derive(Default)]
    struct TestPlatform {
        clipboard: Mutex<Option<PlatformClipboardItem>>,
        writes: Mutex<Vec<PlatformClipboardItem>>,
    }

    impl Platform for TestPlatform {
        fn clipboard_read(&self) -> crate::error::CoreResult<Option<PlatformClipboardItem>> {
            Ok(self.clipboard.lock().expect("lock").clone())
        }

        fn clipboard_write(&self, item: PlatformClipboardItem) -> crate::error::CoreResult<()> {
            self.writes.lock().expect("lock").push(item.clone());
            *self.clipboard.lock().expect("lock") = Some(item);
            Ok(())
        }

        fn clipboard_change_count(&self) -> crate::error::CoreResult<i64> {
            Ok(1)
        }
    }

    #[tokio::test]
    async fn record_and_set_clipboard_use_platform() {
        let platform = Arc::new(TestPlatform {
            clipboard: Mutex::new(Some(PlatformClipboardItem::Text(
                "hello clipboard".to_string(),
            ))),
            writes: Mutex::new(Vec::new()),
        });
        let dir = tempdir().expect("tempdir");
        let workspace = WorkspaceInstance::new_with_platform(dir.path(), platform.clone())
            .await
            .expect("workspace");

        let entry = record_clipboard(&workspace)
            .await
            .expect("record")
            .expect("clipboard entry");
        assert!(matches!(entry.kind, ClipboardKind::Text));
        assert_eq!(entry.text.as_deref(), Some("hello clipboard"));

        set_clipboard_text(&workspace, "updated")
            .await
            .expect("set clipboard");

        let writes = platform.writes.lock().expect("lock");
        assert_eq!(writes.len(), 1);
        assert!(matches!(
            writes.first(),
            Some(PlatformClipboardItem::Text(value)) if value == "updated"
        ));
    }
}
