use std::sync::Arc;

use crate::error::CoreError;
use crate::extension::{boxed_tool_future, Extension, ExtensionKind, ExtensionTool};

#[cfg(target_os = "macos")]
use platform_macos;

#[derive(Debug, Default)]
pub struct SystemExtension;

impl SystemExtension {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Extension for SystemExtension {
    fn id(&self) -> &str {
        "system"
    }

    fn name(&self) -> &str {
        "System"
    }

    fn kind(&self) -> ExtensionKind {
        ExtensionKind::System
    }

    fn tags(&self) -> Vec<String> {
        vec!["system".to_string(), "os".to_string()]
    }

    fn tools(&self) -> Vec<ExtensionTool> {
        #[cfg(target_os = "macos")]
        {
            let list_open_execute = Arc::new(|input: serde_json::Value, _context| {
                boxed_tool_future(async move {
                    let visible_only = input
                        .get("visibleOnly")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    let apps = platform_macos::list_open_apps(visible_only)
                        .map_err(CoreError::Internal)?;
                    Ok(serde_json::to_value(apps).map_err(|error| {
                        CoreError::Internal(format!("failed to serialize open apps: {error}"))
                    })?)
                })
            });
            let list_windows_execute = Arc::new(|input: serde_json::Value, _context| {
                boxed_tool_future(async move {
                    let visible_only = input
                        .get("visibleOnly")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    let snapshot = platform_macos::list_windows_snapshot(visible_only)
                        .map_err(CoreError::Internal)?;
                    Ok(serde_json::json!({
                        "snapshotId": snapshot.snapshot_id,
                        "windows": snapshot.windows,
                    }))
                })
            });
            let run_applescript_execute = Arc::new(|input: serde_json::Value, _context| {
                boxed_tool_future(async move {
                    let script = input
                        .get("script")
                        .and_then(|value| value.as_str())
                        .ok_or_else(|| CoreError::Internal("missing script".to_string()))?;
                    let output =
                        platform_macos::run_applescript(script).map_err(CoreError::Internal)?;
                    Ok(serde_json::json!({ "output": output }))
                })
            });
            let app_action_execute = Arc::new(|input: serde_json::Value, _context| {
                boxed_tool_future(async move {
                    let action = input
                        .get("action")
                        .and_then(|value| value.as_str())
                        .ok_or_else(|| CoreError::Internal("missing action".to_string()))?;
                    let bundle_id = input.get("bundleId").and_then(|value| value.as_str());
                    let pid = input
                        .get("pid")
                        .and_then(|value| value.as_i64())
                        .map(|value| value as i32);
                    if bundle_id.is_none() && pid.is_none() {
                        return Err(CoreError::Internal(
                            "bundleId or pid is required".to_string(),
                        ));
                    }
                    platform_macos::perform_app_action(bundle_id, pid, action)
                        .map_err(CoreError::Internal)?;
                    Ok(serde_json::json!({ "status": "ok" }))
                })
            });
            let window_action_execute = Arc::new(|input: serde_json::Value, _context| {
                boxed_tool_future(async move {
                    let action = input
                        .get("action")
                        .and_then(|value| value.as_str())
                        .ok_or_else(|| CoreError::Internal("missing action".to_string()))?;
                    let window_id = input
                        .get("windowId")
                        .and_then(|value| value.as_u64())
                        .ok_or_else(|| CoreError::Internal("missing windowId".to_string()))?
                        as u32;
                    let snapshot_id = input
                        .get("snapshotId")
                        .and_then(|value| value.as_u64())
                        .or_else(|| input.get("snapshot_id").and_then(|value| value.as_u64()));
                    platform_macos::perform_window_action(window_id, action, snapshot_id)
                        .map_err(CoreError::Internal)?;
                    Ok(serde_json::json!({ "status": "ok" }))
                })
            });
            let list_installed_execute = Arc::new(|_input: serde_json::Value, _context| {
                boxed_tool_future(async move {
                    let apps = platform_macos::list_installed_apps();
                    Ok(serde_json::to_value(apps).map_err(|error| {
                        CoreError::Internal(format!("failed to serialize installed apps: {error}"))
                    })?)
                })
            });

            vec![
                ExtensionTool {
                    id: "list_open_apps".to_string(),
                    name: "List Open Apps".to_string(),
                    description: Some("List running applications and their windows".to_string()),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "visibleOnly": { "type": "boolean", "default": false }
                        },
                        "additionalProperties": false
                    }),
                    execute: list_open_execute,
                },
                ExtensionTool {
                    id: "list_windows".to_string(),
                    name: "List Windows".to_string(),
                    description: Some("List windows from the CG window registry".to_string()),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "visibleOnly": { "type": "boolean", "default": false }
                        },
                        "additionalProperties": false
                    }),
                    execute: list_windows_execute,
                },
                ExtensionTool {
                    id: "run_applescript".to_string(),
                    name: "Run AppleScript".to_string(),
                    description: Some("Run AppleScript automation".to_string()),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "script": { "type": "string" }
                        },
                        "required": ["script"],
                        "additionalProperties": false
                    }),
                    execute: run_applescript_execute,
                },
                ExtensionTool {
                    id: "list_installed_apps".to_string(),
                    name: "List Installed Apps".to_string(),
                    description: Some("List installed applications on this system".to_string()),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {},
                        "additionalProperties": false
                    }),
                    execute: list_installed_execute,
                },
                ExtensionTool {
                    id: "app_action".to_string(),
                    name: "App Action".to_string(),
                    description: Some("Perform an action on an application".to_string()),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "bundleId": { "type": "string" },
                            "pid": { "type": "integer" },
                            "action": {
                                "type": "string",
                                "enum": ["activate", "hide", "quit"]
                            }
                        },
                        "required": ["action"],
                        "additionalProperties": false
                    }),
                    execute: app_action_execute,
                },
                ExtensionTool {
                    id: "window_action".to_string(),
                    name: "Window Action".to_string(),
                    description: Some("Perform an action on a window".to_string()),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "windowId": { "type": "integer" },
                            "snapshotId": { "type": "integer" },
                            "action": {
                                "type": "string",
                                "enum": ["minimize", "close", "focus"]
                            }
                        },
                        "required": ["windowId", "action"],
                        "additionalProperties": false
                    }),
                    execute: window_action_execute,
                },
            ]
        }
        #[cfg(not(target_os = "macos"))]
        {
            let unsupported = |tool_id: &str| {
                let tool_id = tool_id.to_string();
                Arc::new(move |_input: serde_json::Value, _context| {
                    let tool_id = tool_id.clone();
                    boxed_tool_future(async move {
                        Err(CoreError::Internal(format!(
                            "system tool not supported: {tool_id}"
                        )))
                    })
                })
            };

            vec![
                ExtensionTool {
                    id: "list_open_apps".to_string(),
                    name: "List Open Apps".to_string(),
                    description: Some("List running applications and their windows".to_string()),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "visibleOnly": { "type": "boolean", "default": false }
                        },
                        "additionalProperties": false
                    }),
                    execute: unsupported("list_open_apps"),
                },
                ExtensionTool {
                    id: "list_windows".to_string(),
                    name: "List Windows".to_string(),
                    description: Some("List windows from the CG window registry".to_string()),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "visibleOnly": { "type": "boolean", "default": false }
                        },
                        "additionalProperties": false
                    }),
                    execute: unsupported("list_windows"),
                },
                ExtensionTool {
                    id: "run_applescript".to_string(),
                    name: "Run AppleScript".to_string(),
                    description: Some("Run AppleScript automation".to_string()),
                    input_schema: serde_json::json!({
                    "type": "object",
                        "properties": {
                            "script": { "type": "string" }
                        },
                        "required": ["script"],
                        "additionalProperties": false
                    }),
                    execute: unsupported("run_applescript"),
                },
                ExtensionTool {
                    id: "list_installed_apps".to_string(),
                    name: "List Installed Apps".to_string(),
                    description: Some("List installed applications on this system".to_string()),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {},
                        "additionalProperties": false
                    }),
                    execute: unsupported("list_installed_apps"),
                },
                ExtensionTool {
                    id: "app_action".to_string(),
                    name: "App Action".to_string(),
                    description: Some("Perform an action on an application".to_string()),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "bundleId": { "type": "string" },
                            "pid": { "type": "integer" },
                            "action": {
                                "type": "string",
                                "enum": ["activate", "hide", "quit"]
                            }
                        },
                        "required": ["action"],
                        "additionalProperties": false
                    }),
                    execute: unsupported("app_action"),
                },
                ExtensionTool {
                    id: "window_action".to_string(),
                    name: "Window Action".to_string(),
                    description: Some("Perform an action on a window".to_string()),
                    input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "windowId": { "type": "integer" },
                        "snapshotId": { "type": "integer" },
                        "action": {
                            "type": "string",
                            "enum": ["minimize", "close", "focus"]
                        }
                    },
                        "required": ["windowId", "action"],
                        "additionalProperties": false
                    }),
                    execute: unsupported("window_action"),
                },
            ]
        }
    }
}
