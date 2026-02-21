//! TerminalExtension implementation.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{CoreError, CoreResult};
use crate::extension::builtin::manifest_tools::{merge_manifest_tools, parse_builtin_manifest};
use crate::extension::manifest::ExtensionManifest;
use crate::extension::{boxed_tool_future, Extension, ExtensionKind, ExtensionTool};

use super::ops;

pub struct TerminalExtension {
    manifest: ExtensionManifest,
    tools: Vec<ExtensionTool>,
}

impl std::fmt::Debug for TerminalExtension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalExtension").finish()
    }
}

impl Default for TerminalExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalExtension {
    pub fn new() -> Self {
        let manifest = parse_builtin_manifest(include_str!("manifest.json"));
        let mut execute_map = HashMap::new();

        // ── bash ──────────────────────────────────────────────────────
        execute_map.insert(
            "bash",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let command = required_string(&input, "command")?;
                        let timeout_ms = optional_u64(&input, "timeout")
                            .unwrap_or(120_000)
                            .clamp(1, 600_000);
                        let workspace_dir = context.workspace.workspace_dir.clone();
                        let workdir_raw = optional_string(&input, "workdir");
                        let workdir = match workdir_raw {
                            Some(raw) => normalize_path(&raw, &workspace_dir)?,
                            None => workspace_dir,
                        };

                        ops::bash_exec(&command, timeout_ms, &workdir).await
                    })
                },
            ) as _,
        );

        // ── glob ──────────────────────────────────────────────────────
        execute_map.insert(
            "glob",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let pattern = required_string(&input, "pattern")?;
                        let workspace_dir = context.workspace.workspace_dir.clone();
                        let path = match optional_string(&input, "path") {
                            Some(raw) => normalize_path(&raw, &workspace_dir)?,
                            None => workspace_dir,
                        };

                        tokio::task::spawn_blocking(move || ops::glob_files(&pattern, &path))
                            .await
                            .map_err(|e| CoreError::Internal(format!("glob task failed: {e}")))?
                    })
                },
            ) as _,
        );

        // ── grep ──────────────────────────────────────────────────────
        execute_map.insert(
            "grep",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let pattern = required_string(&input, "pattern")?;
                        let workspace_dir = context.workspace.workspace_dir.clone();
                        let path = match optional_string(&input, "path") {
                            Some(raw) => normalize_path(&raw, &workspace_dir)?,
                            None => workspace_dir,
                        };
                        let include = optional_string(&input, "include");

                        tokio::task::spawn_blocking(move || {
                            ops::grep_files(&pattern, &path, include.as_deref())
                        })
                        .await
                        .map_err(|e| CoreError::Internal(format!("grep task failed: {e}")))?
                    })
                },
            ) as _,
        );

        // ── ls ────────────────────────────────────────────────────────
        execute_map.insert(
            "ls",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let workspace_dir = context.workspace.workspace_dir.clone();
                        let path = match optional_string(&input, "path") {
                            Some(raw) => normalize_path(&raw, &workspace_dir)?,
                            None => workspace_dir,
                        };
                        let ignore = optional_string_array(&input, "ignore");

                        tokio::task::spawn_blocking(move || ops::list_dir(&path, &ignore))
                            .await
                            .map_err(|e| CoreError::Internal(format!("ls task failed: {e}")))?
                    })
                },
            ) as _,
        );

        let tools = merge_manifest_tools(&manifest, execute_map);

        Self { manifest, tools }
    }
}

#[async_trait::async_trait]
impl Extension for TerminalExtension {
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

fn optional_string(input: &serde_json::Value, key: &str) -> Option<String> {
    input
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn optional_u64(input: &serde_json::Value, key: &str) -> Option<u64> {
    input.get(key).and_then(|v| v.as_u64())
}

fn optional_string_array(input: &serde_json::Value, key: &str) -> Vec<String> {
    input
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
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
