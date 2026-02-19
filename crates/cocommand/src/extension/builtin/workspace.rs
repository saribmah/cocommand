use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;

use crate::error::CoreError;
use crate::extension::manifest::ExtensionManifest;
use crate::extension::{
    boxed_tool_future, Extension, ExtensionContext, ExtensionKind, ExtensionTool,
};
use crate::llm::{LlmService, LlmSettings};
use crate::utils::time::now_secs;
use crate::workspace::WorkspaceConfig;

use super::manifest_tools::{merge_manifest_tools, parse_builtin_manifest};

pub struct WorkspaceExtension {
    manifest: ExtensionManifest,
    tools: Vec<ExtensionTool>,
}

impl WorkspaceExtension {
    pub fn new(llm: Arc<LlmService>) -> Self {
        let manifest = parse_builtin_manifest(include_str!("workspace_manifest.json"));

        let mut execute_map = HashMap::new();

        execute_map.insert(
            "get_config",
            Arc::new(|_input: serde_json::Value, context: ExtensionContext| {
                boxed_tool_future(async move {
                    let config = context.workspace.config.read().await;
                    serde_json::to_value(&*config).map_err(|e| {
                        CoreError::Internal(format!("failed to serialize config: {e}"))
                    })
                })
            }) as _,
        );

        execute_map.insert(
            "update_config",
            Arc::new(move |input: serde_json::Value, context: ExtensionContext| {
                let llm = llm.clone();
                boxed_tool_future(async move {
                    let payload: WorkspaceConfig =
                        serde_json::from_value(input.get("config").cloned().unwrap_or_default())
                            .map_err(|e| CoreError::InvalidInput(format!("invalid config: {e}")))?;
                    let updated = {
                        let mut config = context.workspace.config.write().await;
                        let mut next = payload;
                        next.version = config.version.clone();
                        next.workspace_id = config.workspace_id.clone();
                        next.created_at = config.created_at;
                        next.last_modified = now_secs();
                        *config = next.clone();
                        next
                    };

                    persist_workspace_config(&context).await?;

                    llm.update_settings(LlmSettings::from_workspace(&updated.llm))
                        .await
                        .map_err(|e| CoreError::Internal(e.to_string()))?;

                    serde_json::to_value(&updated).map_err(|e| {
                        CoreError::Internal(format!("failed to serialize config: {e}"))
                    })
                })
            }) as _,
        );

        execute_map.insert(
            "get_permissions",
            Arc::new(|_input: serde_json::Value, context: ExtensionContext| {
                boxed_tool_future(async move {
                    let snapshot = context.workspace.platform.permissions_snapshot();
                    serde_json::to_value(snapshot).map_err(|error| {
                        CoreError::Internal(format!(
                            "failed to serialize permissions snapshot: {error}"
                        ))
                    })
                })
            }) as _,
        );

        execute_map.insert(
            "open_permission",
            Arc::new(|input: serde_json::Value, context: ExtensionContext| {
                boxed_tool_future(async move {
                    let id = input
                        .get("id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            CoreError::InvalidInput("missing permission id".to_string())
                        })?
                        .to_string();
                    context.workspace.platform.open_permission_settings(&id)?;
                    Ok(json!({ "status": "ok" }))
                })
            }) as _,
        );

        let tools = merge_manifest_tools(&manifest, execute_map);

        Self { manifest, tools }
    }
}

#[async_trait::async_trait]
impl Extension for WorkspaceExtension {
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

async fn persist_workspace_config(context: &ExtensionContext) -> Result<(), CoreError> {
    let value = serde_json::to_value({
        let config = context.workspace.config.read().await;
        config.clone()
    })
    .map_err(|e| CoreError::Internal(format!("failed to serialize config: {e}")))?;
    let workspace_id = {
        let config = context.workspace.config.read().await;
        config.workspace_id.clone()
    };
    context
        .workspace
        .storage
        .write(&["workspace", &workspace_id], &value)
        .await
        .map_err(|e| CoreError::Internal(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tempfile::tempdir;

    use crate::extension::{Extension, ExtensionContext};
    use crate::llm::{LlmService, LlmSettings};
    use crate::platform::{PermissionSnapshot, PermissionStatus, Platform};
    use crate::workspace::WorkspaceInstance;

    use super::WorkspaceExtension;

    #[derive(Default)]
    struct TestPlatform {
        opened_permissions: Mutex<Vec<String>>,
    }

    impl Platform for TestPlatform {
        fn permissions_snapshot(&self) -> PermissionSnapshot {
            PermissionSnapshot {
                platform: "test".to_string(),
                permissions: vec![PermissionStatus {
                    id: "automation".to_string(),
                    label: "Automation".to_string(),
                    granted: true,
                    required: true,
                }],
            }
        }

        fn open_permission_settings(&self, permission: &str) -> crate::error::CoreResult<()> {
            self.opened_permissions
                .lock()
                .expect("lock")
                .push(permission.to_string());
            Ok(())
        }
    }

    fn test_llm_service() -> Arc<LlmService> {
        Arc::new(
            LlmService::new(LlmSettings {
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: None,
                model: "gpt-4o-mini".to_string(),
                system_prompt: "test".to_string(),
                temperature: 0.7,
                max_output_tokens: 1_024,
                max_steps: 4,
            })
            .expect("llm"),
        )
    }

    #[tokio::test]
    async fn permissions_tools_use_platform() {
        let platform = Arc::new(TestPlatform::default());
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(
            WorkspaceInstance::new_with_platform(dir.path(), platform.clone())
                .await
                .expect("workspace"),
        );
        let extension = WorkspaceExtension::new(test_llm_service());

        let get_permissions = extension
            .tools()
            .into_iter()
            .find(|tool| tool.id == "get_permissions")
            .expect("get_permissions tool");
        let get_output = (get_permissions.execute)(
            serde_json::json!({}),
            ExtensionContext {
                workspace: workspace.clone(),
                session_id: "test".to_string(),
            },
        )
        .await
        .expect("get permissions");
        assert_eq!(get_output["platform"], "test");
        assert_eq!(get_output["permissions"][0]["id"], "automation");

        let open_permission = extension
            .tools()
            .into_iter()
            .find(|tool| tool.id == "open_permission")
            .expect("open_permission tool");
        let open_output = (open_permission.execute)(
            serde_json::json!({ "id": "automation" }),
            ExtensionContext {
                workspace,
                session_id: "test".to_string(),
            },
        )
        .await
        .expect("open permission");
        assert_eq!(open_output["status"], "ok");

        let calls = platform.opened_permissions.lock().expect("lock");
        assert_eq!(calls.as_slice(), &["automation".to_string()]);
    }
}
