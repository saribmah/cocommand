use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;

use crate::extension::{Extension, ExtensionContext, ExtensionKind, ExtensionTool};
use crate::server::ServerState;

#[derive(Debug, Serialize)]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub tags: Vec<String>,
    pub tools: Vec<ExtensionToolInfo>,
}

#[derive(Debug, Serialize)]
pub struct ExtensionToolInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct OpenExtensionRequest {
    pub id: String,
}

pub(crate) async fn list_extensions(
    State(state): State<Arc<ServerState>>,
) -> Json<Vec<ExtensionInfo>> {
    let registry = state.workspace.extension_registry.read().await;
    let apps = registry
        .list()
        .into_iter()
        .map(|app| map_extension(app.as_ref()))
        .collect();
    Json(apps)
}

pub(crate) async fn open_extension(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<OpenExtensionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let app_id = payload.id;
    let app = {
        let registry = state.workspace.extension_registry.read().await;
        registry
            .get(&app_id)
            .ok_or((StatusCode::NOT_FOUND, "extension not found".to_string()))?
    };

    let supports_open = app.tools().into_iter().any(|tool| tool.id == "open");
    if !supports_open {
        return Err((
            StatusCode::BAD_REQUEST,
            "extension cannot be opened".to_string(),
        ));
    }

    let session_id = state
        .sessions
        .with_session_mut(|session| {
            let app_id = app_id.clone();
            Box::pin(async move {
                session.activate_extension(&app_id);
                let context = session.context(None).await?;
                Ok(context.session_id)
            })
        })
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    let context = ExtensionContext {
        workspace: Arc::new(state.workspace.clone()),
        session_id,
    };
    let tool = app
        .tools()
        .into_iter()
        .find(|tool| tool.id == "open")
        .ok_or((
            StatusCode::BAD_REQUEST,
            "extension cannot be opened".to_string(),
        ))?;
    let output = (tool.execute)(serde_json::json!({}), context)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    Ok(Json(output))
}

fn map_extension(app: &dyn Extension) -> ExtensionInfo {
    ExtensionInfo {
        id: app.id().to_string(),
        name: app.name().to_string(),
        kind: map_kind(app.kind()),
        tags: app.tags(),
        tools: app.tools().into_iter().map(map_tool).collect(),
    }
}

fn map_tool(tool: ExtensionTool) -> ExtensionToolInfo {
    ExtensionToolInfo {
        id: tool.id,
        name: tool.name,
        description: tool.description,
        input_schema: tool.input_schema,
    }
}

fn map_kind(kind: ExtensionKind) -> String {
    match kind {
        ExtensionKind::System => "system",
        ExtensionKind::BuiltIn => "built-in",
        ExtensionKind::Custom => "custom",
    }
    .to_string()
}
