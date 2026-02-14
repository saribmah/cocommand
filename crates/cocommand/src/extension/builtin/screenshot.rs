use std::any::Any;
use std::path::PathBuf;
use std::sync::Arc;

use crate::error::CoreError;
use crate::extension::{boxed_tool_future, Extension, ExtensionKind, ExtensionTool};
use crate::utils::time::now_secs;

#[cfg(target_os = "macos")]
use platform_macos::{capture_screenshot, ScreenshotMode, ScreenshotOptions};

#[derive(Debug, Default)]
pub struct ScreenshotExtension;

impl ScreenshotExtension {
    pub fn new() -> Self {
        Self
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
}

#[async_trait::async_trait]
impl Extension for ScreenshotExtension {
    fn id(&self) -> &str {
        "screenshot"
    }

    fn name(&self) -> &str {
        "Screenshot"
    }

    fn kind(&self) -> ExtensionKind {
        ExtensionKind::System
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "screenshot".to_string(),
            "screen".to_string(),
            "system".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn tools(&self) -> Vec<ExtensionTool> {
        #[cfg(target_os = "macos")]
        {
            let capture_execute = Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let mode = Self::parse_mode(input.get("mode").and_then(|v| v.as_str()))?;
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
                        let format =
                            Self::normalize_format(input.get("format").and_then(|v| v.as_str()))?;
                        let delay_seconds = input.get("delaySeconds").and_then(|v| v.as_u64());
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
            );

            vec![ExtensionTool {
                id: "capture_screenshot".to_string(),
                name: "Capture Screenshot".to_string(),
                description: Some(
                    "Capture a screenshot using the macOS screencapture tool".to_string(),
                ),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "mode": {
                            "type": "string",
                            "enum": ["interactive", "screen", "window", "rect"],
                            "default": "interactive"
                        },
                        "display": { "type": "integer" },
                        "windowId": { "type": "integer" },
                        "rect": { "type": "string", "description": "x,y,w,h" },
                        "format": {
                            "type": "string",
                            "enum": ["png", "jpg", "tiff", "pdf"],
                            "default": "png"
                        },
                        "delaySeconds": { "type": "integer" },
                        "toClipboard": { "type": "boolean", "default": false },
                        "includeCursor": { "type": "boolean", "default": false }
                    },
                    "additionalProperties": false
                }),
                execute: capture_execute,
            }]
        }

        #[cfg(not(target_os = "macos"))]
        {
            let unsupported = Arc::new(|_input: serde_json::Value, _context| {
                boxed_tool_future(async move {
                    Err(CoreError::Internal(
                        "screenshot tool not supported on this platform".to_string(),
                    ))
                })
            });

            vec![ExtensionTool {
                id: "capture_screenshot".to_string(),
                name: "Capture Screenshot".to_string(),
                description: Some(
                    "Capture a screenshot using the macOS screencapture tool".to_string(),
                ),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "mode": {
                            "type": "string",
                            "enum": ["interactive", "screen", "window", "rect"],
                            "default": "interactive"
                        },
                        "display": { "type": "integer" },
                        "windowId": { "type": "integer" },
                        "rect": { "type": "string", "description": "x,y,w,h" },
                        "format": {
                            "type": "string",
                            "enum": ["png", "jpg", "tiff", "pdf"],
                            "default": "png"
                        },
                        "delaySeconds": { "type": "integer" },
                        "toClipboard": { "type": "boolean", "default": false },
                        "includeCursor": { "type": "boolean", "default": false }
                    },
                    "additionalProperties": false
                }),
                execute: unsupported,
            }]
        }
    }
}
