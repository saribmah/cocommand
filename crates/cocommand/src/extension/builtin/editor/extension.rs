//! EditorExtension implementation.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{CoreError, CoreResult};
use crate::extension::builtin::manifest_tools::{merge_manifest_tools, parse_builtin_manifest};
use crate::extension::manifest::ExtensionManifest;
use crate::extension::{Extension, ExtensionKind, ExtensionTool};

use super::ops;

pub struct EditorExtension {
    manifest: ExtensionManifest,
    tools: Vec<ExtensionTool>,
}

impl std::fmt::Debug for EditorExtension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditorExtension").finish()
    }
}

impl Default for EditorExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorExtension {
    pub fn new() -> Self {
        let manifest = parse_builtin_manifest(include_str!("manifest.json"));
        let mut execute_map = HashMap::new();

        // ── read_file ─────────────────────────────────────────────────
        execute_map.insert(
            "read_file",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    crate::extension::boxed_tool_value_future("Tool result", async move {
                        let file_path = required_string(&input, "file_path")?;
                        let offset = optional_u64(&input, "offset").unwrap_or(1).max(1);
                        let limit = optional_u64(&input, "limit").unwrap_or(2000).max(1);
                        let workspace_dir = context.workspace.workspace_dir.clone();
                        let path = normalize_path(&file_path, &workspace_dir)?;

                        tokio::task::spawn_blocking(move || ops::read_file(&path, offset, limit))
                            .await
                            .map_err(|e| {
                                CoreError::Internal(format!("read_file task failed: {e}"))
                            })?
                    })
                },
            ) as _,
        );

        // ── write_file ────────────────────────────────────────────────
        execute_map.insert(
            "write_file",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    crate::extension::boxed_tool_value_future("Tool result", async move {
                        let file_path = required_string(&input, "file_path")?;
                        let content = required_string_allow_empty(&input, "content")?;
                        let workspace_dir = context.workspace.workspace_dir.clone();
                        let path = normalize_path(&file_path, &workspace_dir)?;

                        tokio::task::spawn_blocking(move || ops::write_file(&path, &content))
                            .await
                            .map_err(|e| {
                                CoreError::Internal(format!("write_file task failed: {e}"))
                            })?
                    })
                },
            ) as _,
        );

        // ── edit_file ─────────────────────────────────────────────────
        execute_map.insert(
            "edit_file",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    crate::extension::boxed_tool_value_future("Tool result", async move {
                        let file_path = required_string(&input, "file_path")?;
                        let old_string = required_string(&input, "old_string")?;
                        let new_string = required_string_allow_empty(&input, "new_string")?;
                        let replace_all = optional_bool(&input, "replace_all").unwrap_or(false);
                        let workspace_dir = context.workspace.workspace_dir.clone();
                        let path = normalize_path(&file_path, &workspace_dir)?;

                        tokio::task::spawn_blocking(move || {
                            ops::edit_file(&path, &old_string, &new_string, replace_all)
                        })
                        .await
                        .map_err(|e| CoreError::Internal(format!("edit_file task failed: {e}")))?
                    })
                },
            ) as _,
        );

        let tools = merge_manifest_tools(&manifest, execute_map);

        Self { manifest, tools }
    }
}

#[async_trait::async_trait]
impl Extension for EditorExtension {
    fn id(&self) -> &str {
        &self.manifest.id
    }

    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn kind(&self) -> ExtensionKind {
        ExtensionKind::System
    }

    fn tags(&self) -> Vec<String> {
        self.manifest
            .routing
            .as_ref()
            .and_then(|r| r.keywords.clone())
            .unwrap_or_default()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn tools(&self) -> Vec<ExtensionTool> {
        self.tools.clone()
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

fn required_string(input: &serde_json::Value, key: &str) -> CoreResult<String> {
    let value = input
        .get(key)
        .and_then(|raw| raw.as_str())
        .ok_or_else(|| CoreError::InvalidInput(format!("missing {key}")))?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(CoreError::InvalidInput(format!("missing {key}")));
    }
    Ok(trimmed.to_string())
}

fn required_string_allow_empty(input: &serde_json::Value, key: &str) -> CoreResult<String> {
    input
        .get(key)
        .and_then(|raw| raw.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| CoreError::InvalidInput(format!("missing {key}")))
}

fn optional_u64(input: &serde_json::Value, key: &str) -> Option<u64> {
    input.get(key).and_then(|v| v.as_u64())
}

fn optional_bool(input: &serde_json::Value, key: &str) -> Option<bool> {
    input.get(key).and_then(|v| v.as_bool())
}

fn normalize_path(raw: &str, workspace_dir: &std::path::Path) -> CoreResult<std::path::PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CoreError::InvalidInput(
            "path must not be empty".to_string(),
        ));
    }

    let candidate = if trimmed == "~" || trimmed.starts_with("~/") || trimmed.starts_with("~\\") {
        let home = std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .map_err(|_| CoreError::Internal("HOME is not set".to_string()))?;
        if trimmed == "~" {
            home
        } else {
            let rest = trimmed
                .strip_prefix("~/")
                .or_else(|| trimmed.strip_prefix("~\\"))
                .unwrap_or_default();
            home.join(rest)
        }
    } else {
        std::path::PathBuf::from(trimmed)
    };

    if candidate.is_absolute() {
        Ok(candidate)
    } else {
        Ok(workspace_dir.join(candidate))
    }
}
