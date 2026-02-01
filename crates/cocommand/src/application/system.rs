use crate::application::{Application, ApplicationKind, ApplicationTool};
use crate::error::CoreError;

#[derive(Debug, Default)]
pub struct SystemApplication;

impl SystemApplication {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Application for SystemApplication {
    fn id(&self) -> &str {
        "system"
    }

    fn name(&self) -> &str {
        "System"
    }

    fn kind(&self) -> ApplicationKind {
        ApplicationKind::System
    }

    fn tags(&self) -> Vec<String> {
        vec!["system".to_string(), "os".to_string()]
    }

    fn tools(&self) -> Vec<ApplicationTool> {
        vec![
            ApplicationTool {
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
            },
            ApplicationTool {
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
            },
            ApplicationTool {
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
            },
            ApplicationTool {
                id: "window_action".to_string(),
                name: "Window Action".to_string(),
                description: Some("Perform an action on a window".to_string()),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "windowId": { "type": "integer" },
                        "action": {
                            "type": "string",
                            "enum": ["minimize", "close", "focus"]
                        }
                    },
                    "required": ["windowId", "action"],
                    "additionalProperties": false
                }),
            },
        ]
    }

    async fn execute(
        &self,
        tool_id: &str,
        input: serde_json::Value,
        _context: &crate::application::ApplicationContext,
    ) -> crate::error::CoreResult<serde_json::Value> {
        #[cfg(target_os = "macos")]
        {
            match tool_id {
                "list_open_apps" => {
                    let visible_only = input
                        .get("visibleOnly")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    let apps = platform_macos::list_open_apps(visible_only)
                        .map_err(CoreError::Internal)?;
                    return Ok(serde_json::to_value(apps).map_err(|error| {
                        CoreError::Internal(format!("failed to serialize open apps: {error}"))
                    })?);
                }
                "run_applescript" => {
                    let script = input
                        .get("script")
                        .and_then(|value| value.as_str())
                        .ok_or_else(|| CoreError::Internal("missing script".to_string()))?;
                    let output = platform_macos::run_applescript(script)
                        .map_err(CoreError::Internal)?;
                    return Ok(serde_json::json!({ "output": output }));
                }
                "app_action" => {
                    let action = input
                        .get("action")
                        .and_then(|value| value.as_str())
                        .ok_or_else(|| CoreError::Internal("missing action".to_string()))?;
                    let bundle_id = input
                        .get("bundleId")
                        .and_then(|value| value.as_str());
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
                    return Ok(serde_json::json!({ "status": "ok" }));
                }
                "window_action" => {
                    let action = input
                        .get("action")
                        .and_then(|value| value.as_str())
                        .ok_or_else(|| CoreError::Internal("missing action".to_string()))?;
                    let window_id = input
                        .get("windowId")
                        .and_then(|value| value.as_u64())
                        .ok_or_else(|| CoreError::Internal("missing windowId".to_string()))?
                        as u32;
                    platform_macos::perform_window_action(window_id, action)
                        .map_err(CoreError::Internal)?;
                    return Ok(serde_json::json!({ "status": "ok" }));
                }
                _ => {}
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = input;
        }
        Err(CoreError::Internal(format!(
            "system tool not supported: {tool_id}"
        )))
    }
}
