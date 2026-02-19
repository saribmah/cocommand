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

#[cfg(target_os = "macos")]
use platform_macos::{
    check_accessibility, check_automation, check_screen_recording, open_permission_settings,
};

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
            Arc::new(
                |_input: serde_json::Value, context: ExtensionContext| {
                    boxed_tool_future(async move {
                        let config = context.workspace.config.read().await;
                        serde_json::to_value(&*config).map_err(|e| {
                            CoreError::Internal(format!("failed to serialize config: {e}"))
                        })
                    })
                },
            ) as _,
        );

        execute_map.insert(
            "update_config",
            Arc::new(
                move |input: serde_json::Value, context: ExtensionContext| {
                    let llm = llm.clone();
                    boxed_tool_future(async move {
                        let payload: WorkspaceConfig = serde_json::from_value(
                            input.get("config").cloned().unwrap_or_default(),
                        )
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
                },
            ) as _,
        );

        execute_map.insert(
            "get_permissions",
            Arc::new(
                |_input: serde_json::Value, _context: ExtensionContext| {
                    boxed_tool_future(async move {
                        #[cfg(target_os = "macos")]
                        {
                            let permissions = vec![
                                json!({
                                    "id": "accessibility",
                                    "label": "Accessibility",
                                    "granted": check_accessibility(),
                                    "required": true,
                                }),
                                json!({
                                    "id": "screen-recording",
                                    "label": "Screen Recording",
                                    "granted": check_screen_recording(),
                                    "required": true,
                                }),
                                json!({
                                    "id": "automation",
                                    "label": "Automation",
                                    "granted": check_automation().unwrap_or(false),
                                    "required": true,
                                }),
                            ];
                            Ok(json!({
                                "platform": "macos",
                                "permissions": permissions,
                            }))
                        }
                        #[cfg(not(target_os = "macos"))]
                        {
                            Ok(json!({
                                "platform": "unsupported",
                                "permissions": [],
                            }))
                        }
                    })
                },
            ) as _,
        );

        execute_map.insert(
            "open_permission",
            Arc::new(
                |input: serde_json::Value, _context: ExtensionContext| {
                    boxed_tool_future(async move {
                        let id = input
                            .get("id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                CoreError::InvalidInput("missing permission id".to_string())
                            })?
                            .to_string();
                        #[cfg(target_os = "macos")]
                        {
                            open_permission_settings(&id)
                                .map_err(|e| CoreError::Internal(e))?;
                            Ok(json!({ "status": "ok" }))
                        }
                        #[cfg(not(target_os = "macos"))]
                        {
                            let _ = id;
                            Err(CoreError::Internal("unsupported platform".to_string()))
                        }
                    })
                },
            ) as _,
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
