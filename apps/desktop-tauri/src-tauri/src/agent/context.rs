//! Agent context building.
//!
//! This module assembles the LLM-facing snapshot and phase information,
//! inspired by opencode's Session/System prompt split. It handles workspace
//! lifecycle rules (staleness, soft-reset, archive) before building context.

use crate::workspace::service::WorkspaceService;
use crate::workspace::types::{WorkspaceSnapshot, WorkspaceState};

use super::session::SessionPhase;

/// Represents the agent's execution context for a turn.
///
/// This struct encapsulates all the information needed to build
/// prompts and configure tools for an agent turn.
#[derive(Debug, Clone)]
pub struct AgentContext {
    /// The current session phase (Control or Execution)
    pub phase: SessionPhase,
    /// The workspace snapshot for the LLM
    pub snapshot: WorkspaceSnapshot,
    /// Whether the workspace was soft-reset for this turn
    pub is_soft_reset: bool,
    /// Whether the workspace is archived
    pub is_archived: bool,
    /// Optional message about workspace state
    pub lifecycle_message: Option<String>,
}

impl AgentContext {
    /// Build context for a fresh/normal workspace
    pub fn normal(phase: SessionPhase, snapshot: WorkspaceSnapshot) -> Self {
        Self {
            phase,
            snapshot,
            is_soft_reset: false,
            is_archived: false,
            lifecycle_message: None,
        }
    }

    /// Build context for a soft-reset workspace
    pub fn soft_reset(
        phase: SessionPhase,
        snapshot: WorkspaceSnapshot,
        idle_hours: u32,
        restorable_apps: usize,
    ) -> Self {
        Self {
            phase,
            snapshot,
            is_soft_reset: true,
            is_archived: false,
            lifecycle_message: Some(format!(
                "Workspace was idle for {}h and has been soft-reset. {} app(s) can be restored. Use window.restore_workspace to recover previous state.",
                idle_hours,
                restorable_apps
            )),
        }
    }

    /// Build context for an archived workspace
    pub fn archived(phase: SessionPhase, idle_hours: u32, restorable_apps: usize) -> Self {
        Self {
            phase,
            snapshot: WorkspaceSnapshot {
                focused_app: None,
                open_apps: vec![],
                staleness: "archived".to_string(),
            },
            is_soft_reset: false,
            is_archived: true,
            lifecycle_message: Some(format!(
                "Workspace is archived (idle {}h, > 7 days). {} app(s) can be restored. Use window.restore_workspace to recover.",
                idle_hours,
                restorable_apps
            )),
        }
    }

    /// Check if the context allows full execution (not archived)
    pub fn can_execute(&self) -> bool {
        !self.is_archived
    }
}

/// Builder for constructing AgentContext with workspace lifecycle checks.
pub struct ContextBuilder<'a> {
    workspace_service: &'a WorkspaceService,
}

impl<'a> ContextBuilder<'a> {
    pub fn new(workspace_service: &'a WorkspaceService) -> Self {
        Self { workspace_service }
    }

    /// Build the agent context for a given workspace state and phase.
    ///
    /// This method applies workspace lifecycle rules:
    /// - Fresh (< 2h): Use workspace as-is
    /// - Stale (2-24h): Refresh ephemeral data, use as-is
    /// - Dormant (24h-7d): Soft reset - empty snapshot for LLM, offer resume
    /// - Archived (> 7d): Minimal snapshot, require explicit restore
    pub fn build(
        &self,
        state: &mut WorkspaceState,
        phase: SessionPhase,
    ) -> AgentContext {
        // Compute current staleness
        let staleness = self.workspace_service.compute_staleness(state);
        let idle_hours = staleness.idle_hours;
        let restorable_apps = self.workspace_service.restorable_app_count(state);

        match staleness.level.as_str() {
            "fresh" => {
                let snapshot = self.workspace_service.snapshot(state);
                AgentContext::normal(phase, snapshot)
            }
            "stale" => {
                // Refresh ephemeral data for stale workspaces
                self.workspace_service.refresh_ephemeral_data(state);
                let snapshot = self.workspace_service.snapshot(state);
                AgentContext::normal(phase, snapshot)
            }
            "dormant" => {
                // Soft reset: provide empty snapshot but preserve internal state
                let snapshot = self.workspace_service.soft_reset_snapshot(state);
                AgentContext::soft_reset(phase, snapshot, idle_hours, restorable_apps)
            }
            "archived" => {
                // Archived: require manual restore
                AgentContext::archived(phase, idle_hours, restorable_apps)
            }
            _ => {
                // Default to normal handling for unknown staleness levels
                let snapshot = self.workspace_service.snapshot(state);
                AgentContext::normal(phase, snapshot)
            }
        }
    }

    /// Build context without modifying state (read-only).
    /// Useful for API endpoints that just need to return a snapshot.
    pub fn build_readonly(&self, state: &WorkspaceState, phase: SessionPhase) -> AgentContext {
        let idle_hours = self.hours_since_last_active(state);
        let restorable_apps = self.workspace_service.restorable_app_count(state);

        // Determine staleness level and build appropriate snapshot with correct staleness
        let staleness_level = if idle_hours >= 168 {
            "archived"
        } else if idle_hours >= 24 {
            "dormant"
        } else if idle_hours >= 2 {
            "stale"
        } else {
            "fresh"
        };

        if idle_hours >= 168 {
            // Archived
            AgentContext::archived(phase, idle_hours, restorable_apps)
        } else if idle_hours >= 24 {
            // Dormant - soft reset
            let snapshot = self.workspace_service.soft_reset_snapshot(state);
            AgentContext::soft_reset(phase, snapshot, idle_hours, restorable_apps)
        } else {
            // Fresh or stale - use normal snapshot but with computed staleness
            let mut snapshot = self.workspace_service.snapshot(state);
            snapshot.staleness = staleness_level.to_string();
            AgentContext::normal(phase, snapshot)
        }
    }

    fn hours_since_last_active(&self, state: &WorkspaceState) -> u32 {
        use time::format_description::well_known::Rfc3339;
        use time::OffsetDateTime;

        let now = OffsetDateTime::now_utc();
        let last_active = OffsetDateTime::parse(&state.last_active_at, &Rfc3339)
            .unwrap_or(now);
        let duration = now - last_active;
        let hours = duration.whole_hours();
        if hours < 0 { 0 } else { hours as u32 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::types::OpenAppSummary;

    #[test]
    fn test_agent_context_normal() {
        let snapshot = WorkspaceSnapshot {
            focused_app: Some("spotify".to_string()),
            open_apps: vec![OpenAppSummary {
                id: "spotify".to_string(),
                summary: "Open".to_string(),
            }],
            staleness: "fresh".to_string(),
        };

        let context = AgentContext::normal(SessionPhase::Control, snapshot);
        assert!(!context.is_soft_reset);
        assert!(!context.is_archived);
        assert!(context.can_execute());
        assert!(context.lifecycle_message.is_none());
    }

    #[test]
    fn test_agent_context_soft_reset() {
        let snapshot = WorkspaceSnapshot {
            focused_app: None,
            open_apps: vec![],
            staleness: "dormant".to_string(),
        };

        let context = AgentContext::soft_reset(SessionPhase::Control, snapshot, 48, 2);
        assert!(context.is_soft_reset);
        assert!(!context.is_archived);
        assert!(context.can_execute());
        assert!(context.lifecycle_message.is_some());
        let msg = context.lifecycle_message.unwrap();
        assert!(msg.contains("48h"));
        assert!(msg.contains("2 app(s)"));
    }

    #[test]
    fn test_agent_context_archived() {
        let context = AgentContext::archived(SessionPhase::Control, 200, 3);
        assert!(!context.is_soft_reset);
        assert!(context.is_archived);
        assert!(!context.can_execute());
        assert!(context.lifecycle_message.is_some());
        let msg = context.lifecycle_message.unwrap();
        assert!(msg.contains("200h"));
        assert!(msg.contains("3 app(s)"));
    }

    #[test]
    fn test_context_builder_fresh_workspace() {
        let workspace_service = WorkspaceService::new();
        let builder = ContextBuilder::new(&workspace_service);

        let mut state = WorkspaceState::default();
        let context = builder.build(&mut state, SessionPhase::Control);

        assert!(!context.is_soft_reset);
        assert!(!context.is_archived);
        assert_eq!(context.snapshot.staleness, "fresh");
    }
}
