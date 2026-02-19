use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::clipboard::set_clipboard_image;
use crate::error::CoreError;
use crate::extension::manifest::ExtensionManifest;
use crate::extension::{boxed_tool_future, Extension, ExtensionKind, ExtensionTool};
use crate::utils::time::now_secs;

use super::manifest_tools::{merge_manifest_tools, parse_builtin_manifest};

#[cfg(target_os = "macos")]
use platform_macos::{capture_screenshot, ScreenshotMode, ScreenshotOptions};

pub struct ScreenshotExtension {
    manifest: ExtensionManifest,
    tools: Vec<ExtensionTool>,
}

impl std::fmt::Debug for ScreenshotExtension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScreenshotExtension").finish()
    }
}

impl Default for ScreenshotExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenshotExtension {
    pub fn new() -> Self {
        let manifest = parse_builtin_manifest(include_str!("screenshot_manifest.json"));
        let execute_map = Self::build_execute_map();
        let tools = merge_manifest_tools(&manifest, execute_map);
        Self { manifest, tools }
    }

    #[cfg(target_os = "macos")]
    fn parse_mode(value: Option<&str>) -> Result<ScreenshotMode, CoreError> {
        match value.unwrap_or("interactive") {
            "interactive" => Ok(ScreenshotMode::Interactive),
            "screen" => Ok(ScreenshotMode::Screen),
            "window" => Ok(ScreenshotMode::Window),
            "rect" => Ok(ScreenshotMode::Rect),
            other => Err(CoreError::Internal(format!(
                "unsupported screenshot mode: {other}"
            ))),
        }
    }

    fn build_output_path(workspace_dir: &PathBuf, session_id: &str, format: &str) -> PathBuf {
        let mut output_dir = workspace_dir.clone();
        output_dir.push("screenshots");
        let filename = format!("{session_id}-{}.{}", now_secs(), format);
        output_dir.push(filename);
        output_dir
    }

    #[cfg(target_os = "macos")]
    fn normalize_format(value: Option<&str>) -> Result<String, CoreError> {
        let format = value.unwrap_or("png");
        match format {
            "png" | "jpg" | "tiff" | "pdf" => Ok(format.to_string()),
            other => Err(CoreError::Internal(format!(
                "unsupported screenshot format: {other}"
            ))),
        }
    }

    fn build_execute_map() -> HashMap<&'static str, crate::extension::ExtensionToolExecute> {
        let mut map = HashMap::new();

        // capture_screenshot
        #[cfg(target_os = "macos")]
        {
            map.insert(
                "capture_screenshot",
                Arc::new(
                    |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                        boxed_tool_future(async move {
                            let mode =
                                Self::parse_mode(input.get("mode").and_then(|v| v.as_str()))?;
                            let display = input
                                .get("display")
                                .and_then(|v| v.as_u64())
                                .map(|v| v as u32);
                            let window_id = input
                                .get("windowId")
                                .and_then(|v| v.as_u64())
                                .map(|v| v as u32);
                            let rect = input
                                .get("rect")
                                .and_then(|v| v.as_str())
                                .map(|v| v.to_string());
                            let format = Self::normalize_format(
                                input.get("format").and_then(|v| v.as_str()),
                            )?;
                            let delay_seconds =
                                input.get("delaySeconds").and_then(|v| v.as_u64());
                            let to_clipboard = input
                                .get("toClipboard")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            let include_cursor = input
                                .get("includeCursor")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);

                            let output_path = if to_clipboard {
                                None
                            } else {
                                let path = Self::build_output_path(
                                    &context.workspace.workspace_dir,
                                    &context.session_id,
                                    &format,
                                );
                                if let Some(parent) = path.parent() {
                                    std::fs::create_dir_all(parent).map_err(|error| {
                                        CoreError::Internal(format!(
                                            "failed to create screenshots directory: {error}"
                                        ))
                                    })?;
                                }
                                Some(path)
                            };

                            let result = capture_screenshot(
                                ScreenshotOptions {
                                    mode,
                                    display,
                                    window_id,
                                    rect,
                                    format: Some(format.clone()),
                                    delay_seconds,
                                    to_clipboard,
                                    include_cursor,
                                },
                                output_path.as_deref(),
                            )
                            .map_err(CoreError::Internal)?;

                            Ok(serde_json::json!({
                                "path": result.path,
                                "filename": result.filename,
                                "format": result.format,
                                "clipboard": result.clipboard
                            }))
                        })
                    },
                ) as _,
            );
        }
        #[cfg(not(target_os = "macos"))]
        {
            map.insert(
                "capture_screenshot",
                Arc::new(|_input: serde_json::Value, _context| {
                    boxed_tool_future(async move {
                        Err(CoreError::Internal(
                            "screenshot tool not supported on this platform".to_string(),
                        ))
                    })
                }) as _,
            );
        }

        // list_screenshots
        map.insert(
            "list_screenshots",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let limit = input
                            .get("limit")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as usize);

                        let screenshots_dir =
                            context.workspace.workspace_dir.join("screenshots");

                        if !screenshots_dir.exists() {
                            return Ok(serde_json::json!([]));
                        }

                        let mut entries = Vec::new();
                        let mut read_dir =
                            tokio::fs::read_dir(&screenshots_dir).await.map_err(|e| {
                                CoreError::Internal(format!(
                                    "failed to read screenshots dir: {e}"
                                ))
                            })?;

                        while let Some(dir_entry) =
                            read_dir.next_entry().await.map_err(|e| {
                                CoreError::Internal(format!("failed to read dir entry: {e}"))
                            })?
                        {
                            let path = dir_entry.path();
                            if !path.is_file() {
                                continue;
                            }
                            let metadata =
                                tokio::fs::metadata(&path).await.map_err(|e| {
                                    CoreError::Internal(format!(
                                        "failed to read metadata: {e}"
                                    ))
                                })?;
                            let filename = path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            let format = path
                                .extension()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            let created_at = metadata
                                .created()
                                .or_else(|_| metadata.modified())
                                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();

                            entries.push(serde_json::json!({
                                "filename": filename,
                                "path": path.to_string_lossy(),
                                "format": format,
                                "size": metadata.len(),
                                "created_at": created_at
                            }));
                        }

                        entries.sort_by(|a, b| {
                            let a_time = a["created_at"].as_u64().unwrap_or(0);
                            let b_time = b["created_at"].as_u64().unwrap_or(0);
                            b_time.cmp(&a_time)
                        });

                        if let Some(limit) = limit {
                            entries.truncate(limit);
                        }

                        Ok(serde_json::Value::Array(entries))
                    })
                },
            ) as _,
        );

        // get_screenshot
        map.insert(
            "get_screenshot",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let filename = input
                            .get("filename")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                CoreError::Internal("filename is required".to_string())
                            })?
                            .to_string();

                        let screenshots_dir =
                            context.workspace.workspace_dir.join("screenshots");
                        let path = screenshots_dir.join(&filename);

                        let resolved = path.canonicalize().map_err(|_| {
                            CoreError::Internal("screenshot not found".to_string())
                        })?;
                        let base = screenshots_dir.canonicalize().map_err(|_| {
                            CoreError::Internal(
                                "screenshots directory not found".to_string(),
                            )
                        })?;
                        if !resolved.starts_with(&base) {
                            return Err(CoreError::Internal(
                                "path traversal denied".to_string(),
                            ));
                        }

                        let metadata =
                            tokio::fs::metadata(&resolved).await.map_err(|_| {
                                CoreError::Internal("screenshot not found".to_string())
                            })?;

                        let format = resolved
                            .extension()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let created_at = metadata
                            .created()
                            .or_else(|_| metadata.modified())
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                            .duration_since(std::time::SystemTime::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();

                        Ok(serde_json::json!({
                            "filename": filename,
                            "path": resolved.to_string_lossy(),
                            "format": format,
                            "size": metadata.len(),
                            "created_at": created_at
                        }))
                    })
                },
            ) as _,
        );

        // delete_screenshot
        map.insert(
            "delete_screenshot",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let filename = input
                            .get("filename")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                CoreError::Internal("filename is required".to_string())
                            })?
                            .to_string();

                        let screenshots_dir =
                            context.workspace.workspace_dir.join("screenshots");
                        let path = screenshots_dir.join(&filename);

                        let resolved = path.canonicalize().map_err(|_| {
                            CoreError::Internal("screenshot not found".to_string())
                        })?;
                        let base = screenshots_dir.canonicalize().map_err(|_| {
                            CoreError::Internal(
                                "screenshots directory not found".to_string(),
                            )
                        })?;
                        if !resolved.starts_with(&base) {
                            return Err(CoreError::Internal(
                                "path traversal denied".to_string(),
                            ));
                        }

                        tokio::fs::remove_file(&resolved).await.map_err(|e| {
                            CoreError::Internal(format!(
                                "failed to delete screenshot: {e}"
                            ))
                        })?;

                        Ok(serde_json::json!({
                            "status": "ok",
                            "deleted": true
                        }))
                    })
                },
            ) as _,
        );

        // copy_screenshot_to_clipboard
        #[cfg(target_os = "macos")]
        {
            map.insert(
                "copy_screenshot_to_clipboard",
                Arc::new(
                    |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                        boxed_tool_future(async move {
                            let filename = input
                                .get("filename")
                                .and_then(|v| v.as_str())
                                .ok_or_else(|| {
                                    CoreError::Internal("filename is required".to_string())
                                })?
                                .to_string();

                            let screenshots_dir =
                                context.workspace.workspace_dir.join("screenshots");
                            let path = screenshots_dir.join(&filename);

                            let resolved = path.canonicalize().map_err(|_| {
                                CoreError::Internal("screenshot not found".to_string())
                            })?;
                            let base = screenshots_dir.canonicalize().map_err(|_| {
                                CoreError::Internal(
                                    "screenshots directory not found".to_string(),
                                )
                            })?;
                            if !resolved.starts_with(&base) {
                                return Err(CoreError::Internal(
                                    "path traversal denied".to_string(),
                                ));
                            }

                            let bytes =
                                tokio::fs::read(&resolved).await.map_err(|e| {
                                    CoreError::Internal(format!(
                                        "failed to read screenshot: {e}"
                                    ))
                                })?;

                            set_clipboard_image(&bytes).await?;

                            Ok(serde_json::json!({
                                "status": "ok"
                            }))
                        })
                    },
                ) as _,
            );
        }
        #[cfg(not(target_os = "macos"))]
        {
            map.insert(
                "copy_screenshot_to_clipboard",
                Arc::new(|_input: serde_json::Value, _context| {
                    boxed_tool_future(async move {
                        Err(CoreError::Internal(
                            "screenshot tool not supported on this platform".to_string(),
                        ))
                    })
                }) as _,
            );
        }

        map
    }
}

#[async_trait::async_trait]
impl Extension for ScreenshotExtension {
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
