use std::any::Any;
use std::sync::Arc;

use serde_json::json;

use crate::error::CoreError;
use crate::extension::{boxed_tool_future, Extension, ExtensionContext, ExtensionKind, ExtensionTool};
use crate::llm::{LlmService, LlmSettings};
use crate::utils::time::now_secs;
use crate::workspace::WorkspaceConfig;

#[cfg(target_os = "macos")]
use platform_macos::{
    check_accessibility, check_automation, check_screen_recording, open_permission_settings,
};

pub struct WorkspaceExtension {
    llm: Arc<LlmService>,
}

impl WorkspaceExtension {
    pub fn new(llm: Arc<LlmService>) -> Self {
        Self { llm }
    }
}

#[async_trait::async_trait]
impl Extension for WorkspaceExtension {
    fn id(&self) -> &str {
        "workspace"
    }

    fn name(&self) -> &str {
        "Workspace"
    }

    fn kind(&self) -> ExtensionKind {
        ExtensionKind::System
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "workspace".to_string(),
            "settings".to_string(),
            "config".to_string(),
            "system".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn tools(&self) -> Vec<ExtensionTool> {
        let get_config_execute = Arc::new(
            |_input: serde_json::Value, context: ExtensionContext| {
                boxed_tool_future(async move {
                    let config = context.workspace.config.read().await;
                    serde_json::to_value(&*config).map_err(|e| {
                        CoreError::Internal(format!("failed to serialize config: {e}"))
                    })
                })
            },
        );

        let llm = self.llm.clone();
        let update_config_execute = Arc::new(
            move |input: serde_json::Value, context: ExtensionContext| {
                let llm = llm.clone();
                boxed_tool_future(async move {
                    let payload: WorkspaceConfig =
                        serde_json::from_value(input.get("config").cloned().unwrap_or_default())
                            .map_err(|e| {
                                CoreError::InvalidInput(format!("invalid config: {e}"))
                            })?;
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
        );

        let get_permissions_execute = Arc::new(
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
        );

        let open_permission_execute = Arc::new(
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
        );

        vec![
            ExtensionTool {
                id: "get_config".to_string(),
                name: "Get Workspace Config".to_string(),
                description: Some("Returns the full workspace configuration".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
                execute: get_config_execute,
            },
            ExtensionTool {
                id: "update_config".to_string(),
                name: "Update Workspace Config".to_string(),
                description: Some(
                    "Updates the workspace configuration. Preserves version, workspace_id, and created_at."
                        .to_string(),
                ),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "config": {
                            "type": "object",
                            "description": "The full WorkspaceConfig to apply"
                        }
                    },
                    "required": ["config"],
                    "additionalProperties": false
                }),
                execute: update_config_execute,
            },
            ExtensionTool {
                id: "get_permissions".to_string(),
                name: "Get Permissions".to_string(),
                description: Some(
                    "Returns macOS permission statuses (accessibility, screen-recording, automation)"
                        .to_string(),
                ),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
                execute: get_permissions_execute,
            },
            ExtensionTool {
                id: "open_permission".to_string(),
                name: "Open Permission Settings".to_string(),
                description: Some(
                    "Opens macOS System Settings for the given permission id".to_string(),
                ),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Permission ID (accessibility, screen-recording, automation)"
                        }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                }),
                execute: open_permission_execute,
            },
        ]
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
