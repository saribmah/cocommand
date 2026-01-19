use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Clone, Serialize, Deserialize)]
pub struct OpenAppState {
    pub id: String,
    #[serde(rename = "openedAt")]
    pub opened_at: String,
    pub panels: Value,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Staleness {
    pub level: String,
    #[serde(rename = "idleHours")]
    pub idle_hours: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub id: String,
    pub version: u32,
    #[serde(rename = "lastActiveAt")]
    pub last_active_at: String,
    #[serde(rename = "focusedApp")]
    pub focused_app: Option<String>,
    #[serde(rename = "openApps")]
    pub open_apps: Vec<OpenAppState>,
    pub staleness: Staleness,
}

#[derive(Clone, Serialize)]
pub struct OpenAppSummary {
    pub id: String,
    pub summary: String,
}

#[derive(Clone, Serialize)]
pub struct WorkspaceSnapshot {
    #[serde(rename = "focusedApp")]
    pub focused_app: Option<String>,
    #[serde(rename = "openApps")]
    pub open_apps: Vec<OpenAppSummary>,
    pub staleness: String,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        WorkspaceState {
            id: "workspace_default".to_string(),
            version: 1,
            last_active_at: now_rfc3339(),
            focused_app: None,
            open_apps: Vec::new(),
            staleness: Staleness {
                level: "fresh".to_string(),
                idle_hours: 0,
            },
        }
    }
}

pub fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string())
}
