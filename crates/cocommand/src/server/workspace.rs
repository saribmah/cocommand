use axum::extract::State;
use axum::Json;
use serde::Serialize;
use std::sync::Arc;

use crate::application::{Application, ApplicationAction, ApplicationKind};
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
