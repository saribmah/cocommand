use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::CoreError;
use crate::extension::ExtensionContext;
use crate::server::error::{ApiError, ApiErrorResponse};
use crate::server::ServerState;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OpenApplicationRequest {
    pub id: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInfo {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationsResponse {
    pub applications: Vec<ApplicationInfo>,
    pub count: usize,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OpenApplicationResponse {
    pub status: String,
    pub opened: bool,
    pub id: String,
}

#[derive(Debug, Deserialize)]
struct InstalledAppToolOutput {
    name: String,
    #[serde(default)]
    bundle_id: Option<String>,
    path: String,
    #[serde(default)]
    icon: Option<String>,
}

#[utoipa::path(
    get,
    path = "/workspace/extension/system/applications",
    tag = "system",
    responses(
        (status = 200, body = ApplicationsResponse),
        (status = 500, body = ApiErrorResponse),
    )
)]
pub(crate) async fn list_applications(
    State(state): State<Arc<ServerState>>,
) -> Result<Json<ApplicationsResponse>, ApiError> {
    let applications = load_installed_applications(state.as_ref()).await?;
    Ok(Json(ApplicationsResponse {
        count: applications.len(),
        applications,
    }))
}

#[utoipa::path(
    post,
    path = "/workspace/extension/system/applications/open",
    tag = "system",
    request_body = OpenApplicationRequest,
    responses(
        (status = 200, body = OpenApplicationResponse),
        (status = 400, body = ApiErrorResponse),
        (status = 404, body = ApiErrorResponse),
    )
)]
pub(crate) async fn open_application(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<OpenApplicationRequest>,
) -> Result<Json<OpenApplicationResponse>, ApiError> {
    let requested_id = payload.id.trim().to_string();
    if requested_id.is_empty() {
        return Err(ApiError::bad_request("missing id"));
    }

    let applications = load_installed_applications(state.as_ref()).await?;
    let app = applications
        .iter()
        .find(|app| {
            app.id == requested_id
                || app.bundle_id.as_deref() == Some(requested_id.as_str())
                || app.path == requested_id
        })
        .ok_or_else(|| ApiError::not_found(format!("application not found: {requested_id}")))?;

    if let Some(bundle_id) = app.bundle_id.as_deref() {
        let app_action_result = execute_system_tool(
            state.as_ref(),
            "app_action",
            serde_json::json!({
                "action": "activate",
                "bundleId": bundle_id,
            }),
        )
        .await;

        if app_action_result.is_ok() {
            return Ok(Json(OpenApplicationResponse {
                status: "ok".to_string(),
                opened: true,
                id: app.id.clone(),
            }));
        }

        let script = format!(
            "tell application id \"{}\" to activate",
            escape_applescript_string(bundle_id)
        );
        execute_system_tool(
            state.as_ref(),
            "run_applescript",
            serde_json::json!({ "script": script }),
        )
        .await?;
        return Ok(Json(OpenApplicationResponse {
            status: "ok".to_string(),
            opened: true,
            id: app.id.clone(),
        }));
    }

    let script = format!(
        "set appPath to POSIX file \"{}\"\ndo shell script \"open -a \" & quoted form of POSIX path of appPath",
        escape_applescript_string(&app.path)
    );
    execute_system_tool(
        state.as_ref(),
        "run_applescript",
        serde_json::json!({ "script": script }),
    )
    .await?;

    Ok(Json(OpenApplicationResponse {
        status: "ok".to_string(),
        opened: true,
        id: app.id.clone(),
    }))
}

async fn load_installed_applications(
    state: &ServerState,
) -> Result<Vec<ApplicationInfo>, ApiError> {
    let output = execute_system_tool(state, "list_installed_apps", serde_json::json!({})).await?;
    let parsed: Vec<InstalledAppToolOutput> = serde_json::from_value(output).map_err(|error| {
        ApiError::internal(format!(
            "invalid system.list_installed_apps output: {error}"
        ))
    })?;

    let mut applications = parsed
        .into_iter()
        .map(|app| {
            let id = app.bundle_id.clone().unwrap_or_else(|| app.path.clone());
            ApplicationInfo {
                id,
                name: app.name,
                bundle_id: app.bundle_id,
                path: app.path,
                icon: app.icon,
            }
        })
        .collect::<Vec<_>>();

    applications.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(applications)
}

async fn execute_system_tool(
    state: &ServerState,
    tool_id: &str,
    input: serde_json::Value,
) -> Result<serde_json::Value, ApiError> {
    let extension = {
        let registry = state.workspace.extension_registry.read().await;
        registry.get("system")
    }
    .ok_or_else(|| ApiError::internal("system extension not found"))?;

    let tool = extension
        .tools()
        .into_iter()
        .find(|tool| tool.id == tool_id)
        .ok_or_else(|| ApiError::internal(format!("system tool not found: {tool_id}")))?;

    let session_id = state
        .sessions
        .with_session_mut(|session| {
            Box::pin(async move {
                let context = session.context(None).await?;
                Ok(context.session_id)
            })
        })
        .await
        .map_err(|e: CoreError| ApiError::from(e))?;

    let context = ExtensionContext {
        workspace: Arc::new(state.workspace.clone()),
        session_id,
    };

    (tool.execute)(input, context)
        .await
        .map_err(|e| ApiError::from(e))
}

fn escape_applescript_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
