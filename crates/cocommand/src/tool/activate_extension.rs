use std::sync::Arc;

use llm_kit_provider_utils::tool::{Tool, ToolExecutionOutput};
use serde_json::json;

use crate::session::SessionManager;
use crate::workspace::WorkspaceInstance;

pub fn build_activate_extension_tool(
    workspace: Arc<WorkspaceInstance>,
    sessions: Arc<SessionManager>,
    session_id: &str,
) -> Tool {
    let session_id = session_id.to_string();
    let execute = Arc::new(move |input: serde_json::Value, _opts| {
        let workspace = workspace.clone();
        let sessions = sessions.clone();
        let session_id = session_id.clone();
        ToolExecutionOutput::Single(Box::pin(async move {
            let app_id = input
                .get("id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| json!({ "error": "missing id" }))?
                .to_string();
            let app = {
                let registry = workspace.extension_registry.read().await;
                registry.get(&app_id)
            }
            .ok_or_else(|| json!({ "error": "extension not found" }))?;
            sessions
                .with_session_mut(|session| {
                    let app_id = app_id.clone();
                    let session_id = session_id.clone();
                    let app = app.clone();
                    let workspace = workspace.clone();
                    Box::pin(async move {
                        if session.session_id != session_id {
                            return Err(crate::error::CoreError::InvalidInput(
                                "session not found".to_string(),
                            ));
                        }
                        let context = crate::extension::ExtensionContext {
                            workspace,
                            session_id: session_id.clone(),
                        };
                        app.initialize(&context).await?;
                        session.activate_extension(&app_id);
                        Ok(())
                    })
                })
                .await
                .map_err(|error| json!({ "error": error.to_string() }))?;
            Ok(json!({ "status": "ok", "activated": true, "id": app_id }))
        }))
    });

    Tool::function(json!({
        "type": "object",
        "properties": {
            "id": { "type": "string" }
        },
        "required": ["id"]
    }))
    .with_description("Activate an extension so its tools become available.")
    .with_execute(execute)
}
