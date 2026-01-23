use serde_json::json;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::types::{now_rfc3339, OpenAppState, OpenAppSummary, Staleness, WorkspaceSnapshot, WorkspaceState};

/// Staleness level thresholds (in hours)
const FRESH_THRESHOLD_HOURS: i64 = 2;
const STALE_THRESHOLD_HOURS: i64 = 24;
const DORMANT_THRESHOLD_HOURS: i64 = 168; // 7 days

#[derive(Clone, Default)]
pub struct WorkspaceService;

impl WorkspaceService {
    pub fn new() -> Self {
        WorkspaceService
    }

    /// Generate a compact snapshot for the LLM context
    pub fn snapshot(&self, state: &WorkspaceState) -> WorkspaceSnapshot {
        let open_apps = state
            .open_apps
            .iter()
            .map(|app| OpenAppSummary {
                id: app.id.clone(),
                summary: self.compute_app_summary(app),
            })
            .collect();

        WorkspaceSnapshot {
            focused_app: state.focused_app.clone(),
            open_apps,
            staleness: state.staleness.level.clone(),
        }
    }

    /// Compute a summary string for an open app
    fn compute_app_summary(&self, app: &OpenAppState) -> String {
        // For now, just return "Open" - in the future this could include
        // panel state information like "Playing Focus playlist"
        if app.panels.is_object() && !app.panels.as_object().map(|o| o.is_empty()).unwrap_or(true) {
            // Could extract state from panels here
            "Active".to_string()
        } else {
            "Open".to_string()
        }
    }

    /// Open an application in the workspace
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

    /// Close an application in the workspace
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

    /// Focus an already-open application
    pub fn focus_app(&self, state: &mut WorkspaceState, app_id: &str) {
        if state.open_apps.iter().any(|app| app.id == app_id) {
            state.focused_app = Some(app_id.to_string());
            self.touch(state);
        }
    }

    /// Update the last active timestamp and reset staleness
    pub fn touch(&self, state: &mut WorkspaceState) {
        state.last_active_at = now_rfc3339();
        state.staleness.level = "fresh".to_string();
        state.staleness.idle_hours = 0;
    }

    /// Compute and update staleness based on time since last activity
    /// Returns the computed staleness
    pub fn compute_staleness(&self, state: &mut WorkspaceState) -> Staleness {
        let idle_hours = self.hours_since_last_active(state);

        let level = if idle_hours < FRESH_THRESHOLD_HOURS as u32 {
            "fresh"
        } else if idle_hours < STALE_THRESHOLD_HOURS as u32 {
            "stale"
        } else if idle_hours < DORMANT_THRESHOLD_HOURS as u32 {
            "dormant"
        } else {
            "archived"
        };

        state.staleness = Staleness {
            level: level.to_string(),
            idle_hours,
        };

        state.staleness.clone()
    }

    /// Calculate hours since last activity
    fn hours_since_last_active(&self, state: &WorkspaceState) -> u32 {
        let now = OffsetDateTime::now_utc();

        let last_active = OffsetDateTime::parse(&state.last_active_at, &Rfc3339)
            .unwrap_or(now);

        let duration = now - last_active;
        let hours = duration.whole_hours();

        if hours < 0 {
            0
        } else {
            hours as u32
        }
    }

    /// Check if workspace should be soft-reset (empty snapshot for LLM)
    /// per virtual-workspace.md inactivity thresholds
    pub fn should_soft_reset(&self, state: &WorkspaceState) -> bool {
        let idle_hours = self.hours_since_last_active(state);
        idle_hours >= STALE_THRESHOLD_HOURS as u32 && idle_hours < DORMANT_THRESHOLD_HOURS as u32
    }

    /// Check if workspace is archived and requires manual restore
    pub fn is_archived(&self, state: &WorkspaceState) -> bool {
        let idle_hours = self.hours_since_last_active(state);
        idle_hours >= DORMANT_THRESHOLD_HOURS as u32
    }

    /// Generate a soft-reset snapshot (empty open apps, offer resume)
    ///
    /// Returns a snapshot with "dormant" staleness level (discrete value)
    /// and includes metadata about restorable apps in the summary.
    pub fn soft_reset_snapshot(&self, _state: &WorkspaceState) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            focused_app: None,
            open_apps: vec![],
            // Use discrete staleness level, not free-form string
            staleness: "dormant".to_string(),
        }
    }

    /// Get the count of apps that can be restored from a soft-reset workspace.
    pub fn restorable_app_count(&self, state: &WorkspaceState) -> usize {
        state.open_apps.len()
    }

    /// Get hours since last activity (public for lifecycle messages).
    pub fn idle_hours(&self, state: &WorkspaceState) -> u32 {
        self.hours_since_last_active(state)
    }

    /// Refresh ephemeral data for apps (called when workspace is stale but < 24h)
    pub fn refresh_ephemeral_data(&self, state: &mut WorkspaceState) {
        // Mark all open apps as needing refresh by clearing their panels
        for app in &mut state.open_apps {
            app.panels = json!({ "stale": true });
        }
        self.touch(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot() {
        let service = WorkspaceService::new();
        let mut state = WorkspaceState::default();

        service.open_app(&mut state, "spotify");
        let snapshot = service.snapshot(&state);

        assert_eq!(snapshot.focused_app, Some("spotify".to_string()));
        assert_eq!(snapshot.open_apps.len(), 1);
        assert_eq!(snapshot.open_apps[0].id, "spotify");
    }

    #[test]
    fn test_open_close_focus() {
        let service = WorkspaceService::new();
        let mut state = WorkspaceState::default();

        service.open_app(&mut state, "spotify");
        service.open_app(&mut state, "calendar");

        assert_eq!(state.open_apps.len(), 2);
        assert_eq!(state.focused_app, Some("calendar".to_string()));

        service.focus_app(&mut state, "spotify");
        assert_eq!(state.focused_app, Some("spotify".to_string()));

        service.close_app(&mut state, "spotify");
        assert_eq!(state.open_apps.len(), 1);
        assert_eq!(state.focused_app, Some("calendar".to_string()));
    }

    #[test]
    fn test_staleness_computation() {
        let service = WorkspaceService::new();
        let mut state = WorkspaceState::default();

        // Fresh state (just created)
        let staleness = service.compute_staleness(&mut state);
        assert_eq!(staleness.level, "fresh");
        assert_eq!(staleness.idle_hours, 0);
    }

    #[test]
    fn test_soft_reset_snapshot() {
        let service = WorkspaceService::new();
        let mut state = WorkspaceState::default();

        service.open_app(&mut state, "spotify");

        let soft_snapshot = service.soft_reset_snapshot(&state);
        assert!(soft_snapshot.focused_app.is_none());
        assert!(soft_snapshot.open_apps.is_empty());
        // Should use discrete staleness level, not free-form string
        assert_eq!(soft_snapshot.staleness, "dormant");
    }

    #[test]
    fn test_restorable_app_count() {
        let service = WorkspaceService::new();
        let mut state = WorkspaceState::default();

        assert_eq!(service.restorable_app_count(&state), 0);

        service.open_app(&mut state, "spotify");
        assert_eq!(service.restorable_app_count(&state), 1);

        service.open_app(&mut state, "calendar");
        assert_eq!(service.restorable_app_count(&state), 2);
    }
}
