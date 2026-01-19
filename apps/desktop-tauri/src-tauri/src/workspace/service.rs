use serde_json::json;

use super::types::{now_rfc3339, OpenAppState, OpenAppSummary, WorkspaceSnapshot, WorkspaceState};

#[derive(Clone, Default)]
pub struct WorkspaceService;

impl WorkspaceService {
    pub fn new() -> Self {
        WorkspaceService
    }

    pub fn snapshot(&self, state: &WorkspaceState) -> WorkspaceSnapshot {
        let open_apps = state
            .open_apps
            .iter()
            .map(|app| OpenAppSummary {
                id: app.id.clone(),
                summary: "Open".to_string(),
            })
            .collect();

        WorkspaceSnapshot {
            focused_app: state.focused_app.clone(),
            open_apps,
            staleness: state.staleness.level.clone(),
        }
    }

    pub fn open_app(&self, state: &mut WorkspaceState, app_id: &str) {
        if !state.open_apps.iter().any(|app| app.id == app_id) {
            state.open_apps.push(OpenAppState {
                id: app_id.to_string(),
                opened_at: now_rfc3339(),
                panels: json!({}),
            });
        }
        state.focused_app = Some(app_id.to_string());
        self.touch(state);
    }

    pub fn close_app(&self, state: &mut WorkspaceState, app_id: &str) {
        state.open_apps.retain(|app| app.id != app_id);
        if state.focused_app.as_deref() == Some(app_id) {
            state.focused_app = state
                .open_apps
                .last()
                .map(|app| app.id.to_string());
        }
        self.touch(state);
    }

    pub fn focus_app(&self, state: &mut WorkspaceState, app_id: &str) {
        if state.open_apps.iter().any(|app| app.id == app_id) {
            state.focused_app = Some(app_id.to_string());
            self.touch(state);
        }
    }

    pub fn touch(&self, state: &mut WorkspaceState) {
        state.last_active_at = now_rfc3339();
        state.staleness.level = "fresh".to_string();
        state.staleness.idle_hours = 0;
    }
}
