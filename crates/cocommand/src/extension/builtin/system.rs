use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::CoreError;
use crate::extension::manifest::ExtensionManifest;
use crate::extension::{boxed_tool_future, Extension, ExtensionKind, ExtensionTool};

use super::manifest_tools::{merge_manifest_tools, parse_builtin_manifest};

#[cfg(target_os = "macos")]
use platform_macos;

pub struct SystemExtension {
    manifest: ExtensionManifest,
    tools: Vec<ExtensionTool>,
}

impl std::fmt::Debug for SystemExtension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemExtension").finish()
    }
}

impl Default for SystemExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemExtension {
    pub fn new() -> Self {
        let manifest = parse_builtin_manifest(include_str!("system_manifest.json"));
        let execute_map = Self::build_execute_map();
        let tools = merge_manifest_tools(&manifest, execute_map);
        Self { manifest, tools }
    }

    fn build_execute_map() -> HashMap<&'static str, crate::extension::ExtensionToolExecute> {
        let mut map = HashMap::new();

        #[cfg(target_os = "macos")]
        {
            map.insert(
                "list_open_apps",
                Arc::new(|input: serde_json::Value, _context| {
                    boxed_tool_future(async move {
                        let visible_only = input
                            .get("visibleOnly")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(false);
                        let apps = platform_macos::list_open_apps(visible_only)
                            .map_err(CoreError::Internal)?;
                        Ok(serde_json::to_value(apps).map_err(|error| {
                            CoreError::Internal(format!(
                                "failed to serialize open apps: {error}"
                            ))
                        })?)
                    })
                }) as _,
            );

            map.insert(
                "list_windows",
                Arc::new(|input: serde_json::Value, _context| {
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
                }) as _,
            );

            map.insert(
                "run_applescript",
                Arc::new(|input: serde_json::Value, _context| {
                    boxed_tool_future(async move {
                        let script = input
                            .get("script")
                            .and_then(|value| value.as_str())
                            .ok_or_else(|| CoreError::Internal("missing script".to_string()))?;
                        let output = platform_macos::run_applescript(script)
                            .map_err(CoreError::Internal)?;
                        Ok(serde_json::json!({ "output": output }))
                    })
                }) as _,
            );

            map.insert(
                "list_installed_apps",
                Arc::new(|_input: serde_json::Value, _context| {
                    boxed_tool_future(async move {
                        let apps = platform_macos::list_installed_apps();
                        Ok(serde_json::to_value(apps).map_err(|error| {
                            CoreError::Internal(format!(
                                "failed to serialize installed apps: {error}"
                            ))
                        })?)
                    })
                }) as _,
            );

            map.insert(
                "app_action",
                Arc::new(|input: serde_json::Value, _context| {
                    boxed_tool_future(async move {
                        let action = input
                            .get("action")
                            .and_then(|value| value.as_str())
                            .ok_or_else(|| CoreError::Internal("missing action".to_string()))?;
                        let bundle_id =
                            input.get("bundleId").and_then(|value| value.as_str());
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
                }) as _,
            );

            map.insert(
                "window_action",
                Arc::new(|input: serde_json::Value, _context| {
                    boxed_tool_future(async move {
                        let action = input
                            .get("action")
                            .and_then(|value| value.as_str())
                            .ok_or_else(|| CoreError::Internal("missing action".to_string()))?;
                        let window_id = input
                            .get("windowId")
                            .and_then(|value| value.as_u64())
                            .ok_or_else(|| {
                                CoreError::Internal("missing windowId".to_string())
                            })? as u32;
                        let snapshot_id = input
                            .get("snapshotId")
                            .and_then(|value| value.as_u64())
                            .or_else(|| {
                                input.get("snapshot_id").and_then(|value| value.as_u64())
                            });
                        platform_macos::perform_window_action(
                            window_id,
                            action,
                            snapshot_id,
                        )
                        .map_err(CoreError::Internal)?;
                        Ok(serde_json::json!({ "status": "ok" }))
                    })
                }) as _,
            );
        }

        #[cfg(not(target_os = "macos"))]
        {
            let unsupported = |tool_id: &'static str| -> crate::extension::ExtensionToolExecute {
                Arc::new(move |_input: serde_json::Value, _context| {
                    let tool_id = tool_id.to_string();
                    boxed_tool_future(async move {
                        Err(CoreError::Internal(format!(
                            "system tool not supported: {tool_id}"
                        )))
                    })
                })
            };

            map.insert("list_open_apps", unsupported("list_open_apps"));
            map.insert("list_windows", unsupported("list_windows"));
            map.insert("run_applescript", unsupported("run_applescript"));
            map.insert("list_installed_apps", unsupported("list_installed_apps"));
            map.insert("app_action", unsupported("app_action"));
            map.insert("window_action", unsupported("window_action"));
        }

        map
    }
}

#[async_trait::async_trait]
impl Extension for SystemExtension {
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
