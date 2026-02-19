use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::CoreError;
use crate::extension::manifest::ExtensionManifest;
use crate::extension::{boxed_tool_future, Extension, ExtensionKind, ExtensionTool};

use super::manifest_tools::{merge_manifest_tools, parse_builtin_manifest};

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

        map.insert(
            "list_open_apps",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let visible_only = input
                            .get("visibleOnly")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(false);
                        let apps = context.workspace.platform.list_open_apps(visible_only)?;
                        Ok(serde_json::to_value(apps).map_err(|error| {
                            CoreError::Internal(format!("failed to serialize open apps: {error}"))
                        })?)
                    })
                },
            ) as _,
        );

        map.insert(
            "list_windows",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let visible_only = input
                            .get("visibleOnly")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(false);
                        let snapshot = context
                            .workspace
                            .platform
                            .list_windows_snapshot(visible_only)?;
                        Ok(serde_json::json!({
                            "snapshotId": snapshot.snapshot_id,
                            "windows": snapshot.windows,
                        }))
                    })
                },
            ) as _,
        );

        map.insert(
            "run_applescript",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let script = input
                            .get("script")
                            .and_then(|value| value.as_str())
                            .ok_or_else(|| CoreError::Internal("missing script".to_string()))?;
                        let output = context.workspace.platform.run_applescript(script)?;
                        Ok(serde_json::json!({ "output": output }))
                    })
                },
            ) as _,
        );

        map.insert(
            "list_installed_apps",
            Arc::new(
                |_input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let apps = context.workspace.platform.list_installed_apps()?;
                        Ok(serde_json::to_value(apps).map_err(|error| {
                            CoreError::Internal(format!(
                                "failed to serialize installed apps: {error}"
                            ))
                        })?)
                    })
                },
            ) as _,
        );

        map.insert(
            "app_action",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
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
                        context
                            .workspace
                            .platform
                            .app_action(bundle_id, pid, action)?;
                        Ok(serde_json::json!({ "status": "ok" }))
                    })
                },
            ) as _,
        );

        map.insert(
            "window_action",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
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
                        context
                            .workspace
                            .platform
                            .window_action(window_id, action, snapshot_id)?;
                        Ok(serde_json::json!({ "status": "ok" }))
                    })
                },
            ) as _,
        );

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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tempfile::tempdir;

    use crate::extension::{Extension, ExtensionContext};
    use crate::platform::{InstalledApp, Platform, SharedPlatform};
    use crate::workspace::WorkspaceInstance;

    use super::SystemExtension;

    struct TestPlatform {
        installed_apps: Vec<InstalledApp>,
    }

    impl Platform for TestPlatform {
        fn list_installed_apps(&self) -> crate::error::CoreResult<Vec<InstalledApp>> {
            Ok(self.installed_apps.clone())
        }
    }

    #[tokio::test]
    async fn list_installed_apps_tool_uses_platform() {
        let platform: SharedPlatform = Arc::new(TestPlatform {
            installed_apps: vec![InstalledApp {
                name: "Finder".to_string(),
                bundle_id: Some("com.apple.finder".to_string()),
                path: "/System/Library/CoreServices/Finder.app".to_string(),
                icon: Some("data:image/png;base64,abc".to_string()),
            }],
        });
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(
            WorkspaceInstance::new_with_platform(dir.path(), platform)
                .await
                .expect("workspace"),
        );

        let extension = SystemExtension::new();
        let tool = extension
            .tools()
            .into_iter()
            .find(|tool| tool.id == "list_installed_apps")
            .expect("tool");

        let output = (tool.execute)(
            serde_json::json!({}),
            ExtensionContext {
                workspace,
                session_id: "test".to_string(),
            },
        )
        .await
        .expect("tool output");

        let apps = output.as_array().expect("array output");
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0]["name"], "Finder");
        assert_eq!(apps[0]["bundle_id"], "com.apple.finder");
    }
}
