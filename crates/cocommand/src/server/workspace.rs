use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;

use crate::application::{Application, ApplicationAction, ApplicationContext, ApplicationKind};
use crate::server::ServerState;

#[derive(Debug, Serialize)]
pub struct ApplicationInfo {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub tags: Vec<String>,
    pub actions: Vec<ApplicationActionInfo>,
}

#[derive(Debug, Serialize)]
pub struct ApplicationActionInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct OpenApplicationRequest {
    pub id: String,
}

pub(crate) async fn list_applications(
    State(state): State<Arc<ServerState>>,
) -> Json<Vec<ApplicationInfo>> {
    let registry = state
        .workspace
        .application_registry
        .read()
        .expect("failed to acquire application registry read lock");
    let apps = registry
        .list()
        .into_iter()
        .map(|app| map_application(app.as_ref()))
        .collect();
    Json(apps)
}

pub(crate) async fn open_application(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<OpenApplicationRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let app_id = payload.id;
    let app = {
        let registry = state
            .workspace
            .application_registry
            .read()
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "registry lock".to_string()))?;
        registry
            .get(&app_id)
            .ok_or((StatusCode::NOT_FOUND, "application not found".to_string()))?
    };

    let supports_open = app.actions().into_iter().any(|action| action.id == "open");
    if !supports_open {
        return Err((StatusCode::BAD_REQUEST, "application cannot be opened".to_string()));
    }

    let session_id = state
        .sessions
        .with_session_mut(|session| {
            let app_id = app_id.clone();
            Box::pin(async move {
                session.open_application(&app_id);
                let context = session.context(None).await?;
                Ok(context.session_id)
            })
        })
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    let context = ApplicationContext {
        workspace: Arc::new(state.workspace.clone()),
        session_id,
    };
    let output = app
        .execute("open", serde_json::json!({}), &context)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    Ok(Json(output))
}

fn map_application(app: &dyn Application) -> ApplicationInfo {
    ApplicationInfo {
        id: app.id().to_string(),
        name: app.name().to_string(),
        kind: map_kind(app.kind()),
        tags: app.tags(),
        actions: app
            .actions()
            .into_iter()
            .map(map_action)
            .collect(),
    }
}

fn map_action(action: ApplicationAction) -> ApplicationActionInfo {
    ApplicationActionInfo {
        id: action.id,
        name: action.name,
        description: action.description,
        input_schema: action.input_schema,
    }
}

fn map_kind(kind: ApplicationKind) -> String {
    match kind {
        ApplicationKind::System => "system",
        ApplicationKind::BuiltIn => "built-in",
        ApplicationKind::Custom => "custom",
    }
    .to_string()
}
