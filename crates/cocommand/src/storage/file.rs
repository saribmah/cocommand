use async_trait::async_trait;
use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::error::{CoreError, CoreResult};
use crate::storage::Storage;

#[derive(Clone)]
pub struct FileStorage {
    root: PathBuf,
}

impl FileStorage {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn build_path(&self, keys: &[&str]) -> CoreResult<PathBuf> {
        if keys.is_empty() {
            return Err(CoreError::InvalidInput("storage keys empty".to_string()));
        }
        let mut path = self.root.clone();
        for key in &keys[..keys.len() - 1] {
            validate_key(key)?;
            path.push(key);
        }
        let mut filename = keys[keys.len() - 1].to_string();
        validate_key(&filename)?;
        if !filename.ends_with(".json") {
            filename.push_str(".json");
        }
        path.push(filename);
        Ok(path)
    }

    async fn ensure_parent_dir(path: &Path) -> CoreResult<()> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|error| {
                    CoreError::Internal(format!(
                        "failed to create storage directory {}: {error}",
                        parent.display()
                    ))
                })?;
        }
        Ok(())
    }
}

#[async_trait]
impl Storage for FileStorage {
    async fn write(&self, keys: &[&str], data: &Value) -> CoreResult<()> {
        let path = self.build_path(keys)?;
        Self::ensure_parent_dir(&path).await?;
        let serialized = serde_json::to_vec_pretty(data)
            .map_err(|error| CoreError::Internal(format!("storage serialize error: {error}")))?;
        tokio::fs::write(&path, serialized)
            .await
            .map_err(|error| {
                CoreError::Internal(format!(
                    "failed to write storage file {}: {error}",
                    path.display()
                ))
            })?;
        Ok(())
    }

    async fn read(&self, keys: &[&str]) -> CoreResult<Option<Value>> {
        let path = self.build_path(keys)?;
        let bytes = match tokio::fs::read(&path).await {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(CoreError::Internal(format!(
                    "failed to read storage file {}: {error}",
                    path.display()
                )))
            }
        };
        let value = serde_json::from_slice(&bytes)
            .map_err(|error| CoreError::Internal(format!("storage parse error: {error}")))?;
        Ok(Some(value))
    }
}

fn validate_key(key: &str) -> CoreResult<()> {
    if key.is_empty() || key == "." || key == ".." {
        return Err(CoreError::InvalidInput(format!(
            "invalid storage key {key}"
        )));
    }
    if key.contains('/') || key.contains('\\') {
        return Err(CoreError::InvalidInput(format!(
            "invalid storage key {key}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn writes_and_reads_json() {
        let dir = tempdir().expect("tempdir");
        let storage = FileStorage::new(dir.path().to_path_buf());
        let value = serde_json::json!({ "hello": "world" });
        storage
            .write(&["session", "abc", "part"], &value)
            .await
            .expect("write");
        let loaded = storage
            .read(&["session", "abc", "part"])
            .await
            .expect("read")
            .expect("value");
        assert_eq!(value, loaded);
    }

    #[tokio::test]
    async fn missing_file_returns_none() {
        let dir = tempdir().expect("tempdir");
        let storage = FileStorage::new(dir.path().to_path_buf());
        let loaded = storage
            .read(&["missing", "value"])
            .await
            .expect("read");
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn invalid_key_rejected() {
        let dir = tempdir().expect("tempdir");
        let storage = FileStorage::new(dir.path().to_path_buf());
        let value = serde_json::json!({ "ok": true });
        let err = storage
            .write(&["..", "bad"], &value)
            .await
            .expect_err("invalid key");
        match err {
            CoreError::InvalidInput(_) => {}
            _ => panic!("expected invalid input"),
        }
    }
}
