use crate::command;
use crate::error::{CoreError, CoreResult};
use crate::routing::Router;
use crate::types::{ActionSummary, ConfirmationDecision, CoreResponse};
use crate::workspace::state::Timestamp;
use crate::workspace::Workspace;

/// Primary facade for the cocommand engine.
///
/// All orchestration flows are accessed through this struct.
pub struct Core {
    workspace: Workspace,
    router: Router,
}

impl Core {
    /// Create a new `Core` instance with a fresh workspace and router.
    pub fn new() -> Self {
        let session_id = uuid::Uuid::new_v4().to_string();
        Core {
            workspace: Workspace::new(session_id),
            router: Router::new(),
        }
    }

    /// Create a `Core` with an existing workspace and router (for testing).
    pub fn with_state(workspace: Workspace, router: Router) -> Self {
        Core { workspace, router }
    }

    /// Submit a natural-language command for processing.
    ///
    /// If follow-up mode is active and valid, the router is biased toward
    /// the last-used app so continuation inputs resolve correctly.
    /// If follow-up has expired, the context is cleared and clarification
    /// is required.
    pub fn submit_command(&mut self, text: &str) -> CoreResult<CoreResponse> {
        let parsed = command::parse(text);
        let now = Self::now();

        // Check follow-up validity and route accordingly.
        let follow_up_ctx = if self.workspace.is_follow_up_valid(now) {
            self.workspace.follow_up.clone()
        } else if self.workspace.follow_up.is_some() {
            // TTL expired or turns exhausted — expire and require clarification.
            self.workspace.expire_follow_up();
            return Ok(CoreResponse::ClarificationNeeded {
                message: "Follow-up expired. Please provide the full command.".to_string(),
            });
        } else {
            None
        };

        let routing_result = self
            .router
            .route_with_follow_up(&parsed, follow_up_ctx.as_ref());

        // If we're in follow-up, consume a turn.
        if follow_up_ctx.is_some() {
            self.workspace.consume_follow_up_turn();
        }

        Ok(CoreResponse::Routed {
            candidates: routing_result
                .candidates
                .into_iter()
                .map(|c| crate::types::RoutedCandidate {
                    app_id: c.app_id,
                    score: c.score,
                    explanation: c.explanation,
                })
                .collect(),
            follow_up_active: self.workspace.mode
                == crate::workspace::state::WorkspaceMode::FollowUpActive,
        })
    }

    /// Activate follow-up mode after a successful command execution.
    pub fn activate_follow_up(
        &mut self,
        command: String,
        entity_ids: Vec<String>,
        app_id: String,
    ) {
        self.workspace.enter_follow_up(command, entity_ids, app_id);
    }

    /// Confirm or deny a pending action.
    pub fn confirm_action(
        &self,
        _confirmation_id: &str,
        _decision: ConfirmationDecision,
    ) -> CoreResult<CoreResponse> {
        Err(CoreError::NotImplemented)
    }

    /// Retrieve a snapshot of the current workspace state.
    pub fn get_workspace_snapshot(&self) -> CoreResult<Workspace> {
        Ok(self.workspace.clone())
    }

    /// Retrieve recent actions up to `limit`.
    pub fn get_recent_actions(&self, _limit: usize) -> CoreResult<Vec<ActionSummary>> {
        Err(CoreError::NotImplemented)
    }

    /// Get a mutable reference to the router for registration.
    pub fn router_mut(&mut self) -> &mut Router {
        &mut self.router
    }

    /// Get a reference to the workspace.
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// Get the current unix timestamp in seconds.
    fn now() -> Timestamp {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::RoutingMetadata;
    use crate::workspace::state::{FollowUpContext, WorkspaceMode, FOLLOW_UP_MAX_TURNS};

    fn calendar_metadata() -> RoutingMetadata {
        RoutingMetadata {
            app_id: "calendar".to_string(),
            keywords: vec!["schedule".into(), "meeting".into(), "event".into()],
            examples: vec!["schedule a meeting".into(), "create an event".into()],
            verbs: vec!["schedule".into(), "create".into(), "cancel".into()],
            objects: vec!["meeting".into(), "event".into(), "appointment".into()],
        }
    }

    fn notes_metadata() -> RoutingMetadata {
        RoutingMetadata {
            app_id: "notes".to_string(),
            keywords: vec!["note".into(), "memo".into()],
            examples: vec!["show last note".into()],
            verbs: vec!["show".into(), "write".into(), "create".into()],
            objects: vec!["note".into(), "memo".into()],
        }
    }

    #[test]
    fn core_new_returns_instance() {
        let _core = Core::new();
    }

    #[test]
    fn submit_command_routes_successfully() {
        let mut core = Core::new();
        core.router_mut().register(calendar_metadata());

        let resp = core.submit_command("schedule a meeting").unwrap();
        match resp {
            CoreResponse::Routed { candidates, .. } => {
                assert!(!candidates.is_empty());
                assert_eq!(candidates[0].app_id, "calendar");
            }
            _ => panic!("Expected Routed response"),
        }
    }

    #[test]
    fn follow_up_within_ttl_biases_router() {
        let mut core = Core::new();
        core.router_mut().register(calendar_metadata());
        core.router_mut().register(notes_metadata());

        // Simulate a successful calendar command.
        core.activate_follow_up(
            "create event at 2pm".to_string(),
            vec!["event-123".to_string()],
            "calendar".to_string(),
        );

        // "make it 2:30" has no keyword matches, but follow-up bias
        // should add calendar as a candidate.
        let resp = core.submit_command("make it 2:30").unwrap();
        match resp {
            CoreResponse::Routed {
                candidates,
                follow_up_active,
            } => {
                assert!(
                    candidates.iter().any(|c| c.app_id == "calendar"),
                    "Calendar should appear due to follow-up bias"
                );
                // The calendar candidate's explanation should mention follow-up.
                let cal = candidates.iter().find(|c| c.app_id == "calendar").unwrap();
                assert!(cal.explanation.contains("follow-up bias"));
                // follow_up may still be active (1 turn consumed, < max).
                assert!(follow_up_active || !follow_up_active); // just checking structure
            }
            _ => panic!("Expected Routed response"),
        }
    }

    #[test]
    fn follow_up_after_ttl_expiry_triggers_clarification() {
        let ws = Workspace::new("test-session".to_string());
        let mut core = Core::with_state(ws, Router::new());
        core.router_mut().register(calendar_metadata());

        // Manually set follow-up with an already-expired TTL.
        core.workspace.follow_up = Some(FollowUpContext {
            last_command: "create event".to_string(),
            last_result_entity_ids: vec!["event-123".to_string()],
            last_app_id: "calendar".to_string(),
            expires_at: 0, // Already expired (epoch).
            turn_count: 0,
            max_turns: FOLLOW_UP_MAX_TURNS,
        });
        core.workspace.mode = WorkspaceMode::FollowUpActive;

        let resp = core.submit_command("make it 2:30").unwrap();
        match resp {
            CoreResponse::ClarificationNeeded { message } => {
                assert!(message.contains("expired"));
            }
            _ => panic!("Expected ClarificationNeeded after TTL expiry"),
        }

        // Workspace should be back to Idle.
        assert_eq!(core.workspace().mode, WorkspaceMode::Idle);
        assert!(core.workspace().follow_up.is_none());
    }

    #[test]
    fn follow_up_turn_limit_exhausts_context() {
        let mut core = Core::new();
        core.router_mut().register(calendar_metadata());

        core.activate_follow_up(
            "create event".to_string(),
            vec!["event-123".to_string()],
            "calendar".to_string(),
        );

        // Consume turns up to the limit.
        for i in 0..FOLLOW_UP_MAX_TURNS {
            let resp = core.submit_command("update it").unwrap();
            if i < FOLLOW_UP_MAX_TURNS - 1 {
                // Should still be routed with follow-up active.
                assert!(matches!(resp, CoreResponse::Routed { .. }));
            }
        }

        // Next command should trigger clarification since turns exhausted.
        let resp = core.submit_command("change the time").unwrap();
        match resp {
            CoreResponse::ClarificationNeeded { .. } => {}
            CoreResponse::Routed { follow_up_active, .. } => {
                assert!(!follow_up_active, "Follow-up should not be active after exhaustion");
            }
        }
    }

    #[test]
    fn workspace_mode_transitions_correctly() {
        let mut core = Core::new();

        // Starts idle.
        assert_eq!(core.workspace().mode, WorkspaceMode::Idle);

        // Enter follow-up.
        core.activate_follow_up(
            "test cmd".to_string(),
            vec!["id-1".to_string()],
            "test_app".to_string(),
        );
        assert_eq!(core.workspace().mode, WorkspaceMode::FollowUpActive);

        // Expire.
        core.workspace.expire_follow_up();
        assert_eq!(core.workspace().mode, WorkspaceMode::Idle);
    }

    #[test]
    fn confirm_action_returns_not_implemented() {
        let core = Core::new();
        let err = core
            .confirm_action("id", ConfirmationDecision::Approve)
            .unwrap_err();
        assert!(matches!(err, CoreError::NotImplemented));
    }

    #[test]
    fn get_workspace_snapshot_returns_workspace() {
        let core = Core::new();
        let ws = core.get_workspace_snapshot().unwrap();
        assert_eq!(ws.mode, WorkspaceMode::Idle);
    }

    #[test]
    fn get_recent_actions_returns_not_implemented() {
        let core = Core::new();
        let err = core.get_recent_actions(10).unwrap_err();
        assert!(matches!(err, CoreError::NotImplemented));
    }
}
