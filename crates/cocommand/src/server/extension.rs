use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::extension::{Extension, ExtensionContext, ExtensionKind, ExtensionStatus, ExtensionTool};
use crate::server::error::{ApiError, ApiErrorResponse};
use crate::server::ServerState;

#[derive(Debug, Serialize, ToSchema)]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub status: String,
    pub tags: Vec<String>,
    pub tools: Vec<ExtensionToolInfo>,
    pub view: Option<ExtensionViewInfo>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ExtensionViewInfo {
    pub entry: String,
    pub label: String,
    pub popout: Option<ExtensionViewPopoutInfo>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ExtensionViewPopoutInfo {
    pub width: u32,
    pub height: u32,
    pub title: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ExtensionToolInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct OpenExtensionRequest {
    pub id: String,
}

#[utoipa::path(
    get,
    path = "/workspace/extensions",
    tag = "extensions",
    responses(
        (status = 200, body = Vec<ExtensionInfo>),
    )
)]
pub(crate) async fn list_extensions(
    State(state): State<Arc<ServerState>>,
) -> Json<Vec<ExtensionInfo>> {
    let registry = state.workspace.extension_registry.read().await;
    let config = state.workspace.config.read().await;
    let installed = &config.extensions.installed;

    let apps = registry
        .list()
        .into_iter()
        .map(|app| {
            let mut info = map_extension(app.as_ref());
            // Override status to "disabled" if config says so
            let is_disabled = installed
                .iter()
                .find(|e| e.extension_id == info.id)
                .is_some_and(|e| !e.enabled);
            if is_disabled {
                info.status = "disabled".to_string();
            }
            info
        })
        .collect();
    Json(apps)
}

#[utoipa::path(
    post,
    path = "/workspace/extensions/open",
    tag = "extensions",
    request_body = OpenExtensionRequest,
    responses(
        (status = 200, description = "Tool output from the extension's open tool"),
        (status = 400, body = ApiErrorResponse),
        (status = 404, body = ApiErrorResponse),
    )
)]
pub(crate) async fn open_extension(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<OpenExtensionRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app_id = payload.id;
    let app = {
        let registry = state.workspace.extension_registry.read().await;
        registry
            .get(&app_id)
            .ok_or_else(|| ApiError::not_found("extension not found"))?
    };

    let supports_open = app.tools().into_iter().any(|tool| tool.id == "open");
    if !supports_open {
        return Err(ApiError::bad_request("extension cannot be opened"));
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
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let context = ExtensionContext {
        workspace: Arc::new(state.workspace.clone()),
        session_id,
    };
    let tool = app
        .tools()
        .into_iter()
        .find(|tool| tool.id == "open")
        .ok_or_else(|| ApiError::bad_request("extension cannot be opened"))?;
    let output = (tool.execute)(serde_json::json!({}), context)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok(Json(output))
}

fn map_extension(app: &dyn Extension) -> ExtensionInfo {
    ExtensionInfo {
        id: app.id().to_string(),
        name: app.name().to_string(),
        kind: map_kind(app.kind()),
        status: map_status(app.status()),
        tags: app.tags(),
        tools: app.tools().into_iter().map(map_tool).collect(),
        view: app.view_config().map(map_view_config),
    }
}

fn map_view_config(config: &crate::extension::manifest::ViewConfig) -> ExtensionViewInfo {
    ExtensionViewInfo {
        entry: config.entry.clone(),
        label: config.label.clone(),
        popout: config.popout.as_ref().map(|p| ExtensionViewPopoutInfo {
            width: p.width,
            height: p.height,
            title: p.title.clone(),
        }),
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

fn map_status(status: ExtensionStatus) -> String {
    match status {
        ExtensionStatus::Ready => "ready",
        ExtensionStatus::Building => "building",
        ExtensionStatus::Error => "error",
        ExtensionStatus::Disabled => "disabled",
    }
    .to_string()
}

fn map_kind(kind: ExtensionKind) -> String {
    match kind {
        ExtensionKind::System => "system",
        ExtensionKind::BuiltIn => "built-in",
        ExtensionKind::Custom => "custom",
    }
    .to_string()
}
