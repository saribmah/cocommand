use std::time::SystemTime;
use uuid::Uuid;

use crate::command;
use crate::error::CoreResult;
use crate::events::Event;
use crate::routing::Router;
use crate::storage::Storage;
use crate::types::{ActionSummary, ArtifactAction, CoreResponse, RoutedCandidate};
use crate::workspace::state::Timestamp;
use crate::workspace::Workspace;

/// Primary facade for the cocommand engine.
///
/// All orchestration flows are accessed through this struct.
/// Responses are returned as [`CoreResponse`] — the single stable shape
/// used across the Tauri boundary.
pub struct Core {
    workspace: Workspace,
    router: Router,
    storage: Box<dyn Storage>,
}

impl Core {
    /// Create a new `Core` instance with the given storage backend.
    pub fn new(storage: Box<dyn Storage>) -> Self {
        let session_id = Uuid::new_v4().to_string();
        Core {
            workspace: Workspace::new(session_id),
            router: Router::new(),
            storage,
        }
    }

    /// Create a `Core` with an existing workspace, router, and storage (for testing).
    pub fn with_state(workspace: Workspace, router: Router, storage: Box<dyn Storage>) -> Self {
        Core { workspace, router, storage }
    }

    /// Submit a natural-language command for processing.
    ///
    /// Returns a [`CoreResponse`] — either an `Artifact` with the routing
    /// result, or an `Error` if follow-up context has expired.
    /// Read-only verbs that produce a Preview response instead of an Artifact.
    const PREVIEW_VERBS: &'static [&'static str] = &["show", "view", "get", "display", "preview", "read"];

    pub fn submit_command(&mut self, text: &str) -> CoreResult<CoreResponse> {
        // Log the user message event.
        self.storage.event_log_mut().append(Event::UserMessage {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            text: text.to_string(),
        });

        // If a confirmation is pending, the user must resolve it first.
        if let Some(cp) = &self.workspace.confirmation_pending {
            return Ok(CoreResponse::Confirmation {
                confirmation_id: cp.confirmation_id.clone(),
                prompt: format!("Pending action: {}", cp.tool_id),
                description: "Please confirm or deny before submitting new commands.".to_string(),
            });
        }

        let parsed = command::parse(text);
        let now = Self::now();

        // Check follow-up validity and route accordingly.
        let follow_up_ctx = if self.workspace.is_follow_up_valid(now) {
            self.workspace.follow_up.clone()
        } else if self.workspace.follow_up.is_some() {
            // TTL expired or turns exhausted — expire and return error.
            self.workspace.expire_follow_up();
            return Ok(CoreResponse::Error {
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

        // Build the artifact content from routing candidates.
        let candidates: Vec<RoutedCandidate> = routing_result
            .candidates
            .into_iter()
            .map(|c| RoutedCandidate {
                app_id: c.app_id,
                score: c.score,
                explanation: c.explanation,
            })
            .collect();

        if candidates.is_empty() {
            return Ok(CoreResponse::Artifact {
                content: "No matching capabilities found.".to_string(),
                actions: vec![],
            });
        }

        // If the command starts with a read-only verb, return Preview.
        if Self::is_preview_command(&parsed.normalized_text) {
            let top = &candidates[0];
            return Ok(CoreResponse::Preview {
                title: format!("{} result", top.app_id),
                content: top.explanation.clone(),
            });
        }

        let top = &candidates[0];
        let content = format!(
            "Routed to {} (score: {:.1}): {}",
            top.app_id, top.score, top.explanation
        );

        // Build actions based on current workspace state.
        let actions = self.build_artifact_actions();

        Ok(CoreResponse::Artifact { content, actions })
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
    ///
    /// Looks up the pending confirmation in the workspace by ID.
    /// Returns an `Artifact` on success or an `Error` if no matching
    /// confirmation is found.
    pub fn confirm_action(
        &mut self,
        confirmation_id: &str,
        decision: bool,
    ) -> CoreResult<CoreResponse> {
        let pending = self.workspace.confirmation_pending.as_ref();

        match pending {
            Some(cp) if cp.confirmation_id == confirmation_id => {
                let tool_id = cp.tool_id.clone();
                self.workspace.clear_confirmation();

                if decision {
                    Ok(CoreResponse::Artifact {
                        content: format!("Action confirmed: {tool_id}"),
                        actions: vec![],
                    })
                } else {
                    Ok(CoreResponse::Artifact {
                        content: format!("Action cancelled: {tool_id}"),
                        actions: vec![],
                    })
                }
            }
            Some(_) => Ok(CoreResponse::Error {
                message: format!(
                    "No pending confirmation with id '{confirmation_id}'."
                ),
            }),
            None => Ok(CoreResponse::Error {
                message: "No action pending confirmation.".to_string(),
            }),
        }
    }

    /// Retrieve a snapshot of the current workspace state.
    pub fn get_workspace_snapshot(&self) -> CoreResult<Workspace> {
        Ok(self.workspace.clone())
    }

    /// Retrieve recent actions up to `limit`.
    ///
    /// Returns pre-computed summaries from the event log. Summaries are
    /// derived from structural metadata at write time and never contain
    /// raw user text or sensitive content.
    pub fn get_recent_actions(&self, limit: usize) -> CoreResult<Vec<ActionSummary>> {
        let records = self.storage.event_log().tail(limit);
        let summaries = records
            .into_iter()
            .map(|record| ActionSummary {
                id: record.event.id().to_string(),
                description: record.summary,
            })
            .collect();
        Ok(summaries)
    }

    /// Get a reference to the storage backend.
    pub fn storage(&self) -> &dyn Storage {
        &*self.storage
    }

    /// Get a mutable reference to the storage backend.
    pub fn storage_mut(&mut self) -> &mut dyn Storage {
        &mut *self.storage
    }

    /// Get a mutable reference to the router for registration.
    pub fn router_mut(&mut self) -> &mut Router {
        &mut self.router
    }

    /// Get a reference to the workspace.
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// Get a mutable reference to the workspace (for testing).
    #[cfg(test)]
    pub fn workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspace
    }

    /// Get the current unix timestamp in seconds.
    fn now() -> Timestamp {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Check if the command text starts with a read-only verb.
    fn is_preview_command(text: &str) -> bool {
        let first_word = text.split_whitespace().next().unwrap_or("");
        Self::PREVIEW_VERBS
            .iter()
            .any(|v| first_word.eq_ignore_ascii_case(v))
    }

    /// Build artifact actions based on the current workspace mode.
    fn build_artifact_actions(&self) -> Vec<ArtifactAction> {
        if let Some(cp) = &self.workspace.confirmation_pending {
            vec![ArtifactAction {
                id: cp.confirmation_id.clone(),
                label: format!("Confirm: {}", cp.tool_id),
            }]
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::RoutingMetadata;
    use crate::storage::MemoryStorage;
    use crate::workspace::state::{
        ConfirmationPending, FollowUpContext, WorkspaceMode, FOLLOW_UP_MAX_TURNS,
    };

    fn make_storage() -> Box<dyn Storage> {
        Box::new(MemoryStorage::new())
    }

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
        let _core = Core::new(make_storage());
    }

    #[test]
    fn submit_command_returns_artifact() {
        let mut core = Core::new(make_storage());
        core.router_mut().register(calendar_metadata());

        let resp = core.submit_command("schedule a meeting").unwrap();
        match resp {
            CoreResponse::Artifact { content, actions } => {
                assert!(content.contains("calendar"));
                assert!(actions.is_empty());
            }
            _ => panic!("Expected Artifact response"),
        }
    }

    #[test]
    fn submit_command_no_candidates_returns_artifact() {
        let mut core = Core::new(make_storage());
        // No apps registered — should still return Artifact with "no match" message.
        let resp = core.submit_command("do something unknown").unwrap();
        match resp {
            CoreResponse::Artifact { content, .. } => {
                assert!(content.contains("No matching"));
            }
            _ => panic!("Expected Artifact response"),
        }
    }

    #[test]
    fn submit_command_preview_verb_returns_preview() {
        let mut core = Core::new(make_storage());
        core.router_mut().register(notes_metadata());

        let resp = core.submit_command("show last note").unwrap();
        match resp {
            CoreResponse::Preview { title, content } => {
                assert!(title.contains("notes"));
                assert!(!content.is_empty());
            }
            _ => panic!("Expected Preview for read-only verb, got {:?}", resp),
        }
    }

    #[test]
    fn submit_command_pending_confirmation_returns_confirmation() {
        let mut core = Core::new(make_storage());
        core.router_mut().register(calendar_metadata());

        core.workspace.confirmation_pending = Some(ConfirmationPending {
            confirmation_id: "confirm-pending".to_string(),
            tool_id: "dangerous_op".to_string(),
            args: serde_json::json!({}),
            requested_at: 500,
        });

        let resp = core.submit_command("schedule a meeting").unwrap();
        match resp {
            CoreResponse::Confirmation {
                confirmation_id,
                prompt,
                ..
            } => {
                assert_eq!(confirmation_id, "confirm-pending");
                assert!(prompt.contains("dangerous_op"));
            }
            _ => panic!("Expected Confirmation when pending, got {:?}", resp),
        }
    }

    #[test]
    fn follow_up_within_ttl_biases_router() {
        let mut core = Core::new(make_storage());
        core.router_mut().register(calendar_metadata());
        core.router_mut().register(notes_metadata());

        core.activate_follow_up(
            "create event at 2pm".to_string(),
            vec!["event-123".to_string()],
            "calendar".to_string(),
        );

        let resp = core.submit_command("make it 2:30").unwrap();
        match resp {
            CoreResponse::Artifact { content, .. } => {
                assert!(
                    content.contains("calendar"),
                    "Calendar should appear due to follow-up bias"
                );
            }
            _ => panic!("Expected Artifact response"),
        }
    }

    #[test]
    fn follow_up_after_ttl_expiry_triggers_error() {
        let ws = Workspace::new("test-session".to_string());
        let mut core = Core::with_state(ws, Router::new(), make_storage());
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
            CoreResponse::Error { message } => {
                assert!(message.contains("expired"));
            }
            _ => panic!("Expected Error after TTL expiry"),
        }

        assert_eq!(core.workspace().mode, WorkspaceMode::Idle);
        assert!(core.workspace().follow_up.is_none());
    }

    #[test]
    fn follow_up_turn_limit_exhausts_context() {
        let mut core = Core::new(make_storage());
        core.router_mut().register(calendar_metadata());

        core.activate_follow_up(
            "create event".to_string(),
            vec!["event-123".to_string()],
            "calendar".to_string(),
        );

        // Consume turns up to the limit.
        for _ in 0..FOLLOW_UP_MAX_TURNS {
            let resp = core.submit_command("update it").unwrap();
            assert!(matches!(resp, CoreResponse::Artifact { .. }));
        }

        // Next command should trigger error since turns exhausted.
        let resp = core.submit_command("change the time").unwrap();
        match resp {
            CoreResponse::Error { message } => {
                assert!(message.contains("expired"));
            }
            CoreResponse::Artifact { .. } => {
                // Follow-up already expired; workspace mode should be Idle.
                assert_eq!(core.workspace().mode, WorkspaceMode::Idle);
            }
            _ => panic!("Expected Error or Artifact after exhaustion"),
        }
    }

    #[test]
    fn workspace_mode_transitions_correctly() {
        let mut core = Core::new(make_storage());

        assert_eq!(core.workspace().mode, WorkspaceMode::Idle);

        core.activate_follow_up(
            "test cmd".to_string(),
            vec!["id-1".to_string()],
            "test_app".to_string(),
        );
        assert_eq!(core.workspace().mode, WorkspaceMode::FollowUpActive);

        core.workspace.expire_follow_up();
        assert_eq!(core.workspace().mode, WorkspaceMode::Idle);
    }

    #[test]
    fn confirm_action_approve_returns_artifact() {
        let mut core = Core::new(make_storage());
        core.workspace.confirmation_pending = Some(ConfirmationPending {
            confirmation_id: "confirm-abc".to_string(),
            tool_id: "delete_file".to_string(),
            args: serde_json::json!({"path": "/tmp/test"}),
            requested_at: 1000,
        });
        core.workspace.mode = WorkspaceMode::AwaitingConfirmation;

        let resp = core.confirm_action("confirm-abc", true).unwrap();
        match resp {
            CoreResponse::Artifact { content, .. } => {
                assert!(content.contains("confirmed"));
                assert!(content.contains("delete_file"));
            }
            _ => panic!("Expected Artifact for approved action"),
        }
        assert!(core.workspace().confirmation_pending.is_none());
    }

    #[test]
    fn confirm_action_deny_returns_artifact() {
        let mut core = Core::new(make_storage());
        core.workspace.confirmation_pending = Some(ConfirmationPending {
            confirmation_id: "confirm-xyz".to_string(),
            tool_id: "rm_dir".to_string(),
            args: serde_json::json!({}),
            requested_at: 2000,
        });
        core.workspace.mode = WorkspaceMode::AwaitingConfirmation;

        let resp = core.confirm_action("confirm-xyz", false).unwrap();
        match resp {
            CoreResponse::Artifact { content, .. } => {
                assert!(content.contains("cancelled"));
                assert!(content.contains("rm_dir"));
            }
            _ => panic!("Expected Artifact for denied action"),
        }
    }

    #[test]
    fn confirm_action_wrong_id_returns_error() {
        let mut core = Core::new(make_storage());
        core.workspace.confirmation_pending = Some(ConfirmationPending {
            confirmation_id: "confirm-abc".to_string(),
            tool_id: "test_tool".to_string(),
            args: serde_json::json!({}),
            requested_at: 1000,
        });

        let resp = core.confirm_action("wrong-id", true).unwrap();
        match resp {
            CoreResponse::Error { message } => {
                assert!(message.contains("wrong-id"));
            }
            _ => panic!("Expected Error for wrong confirmation_id"),
        }
    }

    #[test]
    fn confirm_action_no_pending_returns_error() {
        let mut core = Core::new(make_storage());

        let resp = core.confirm_action("any-id", true).unwrap();
        match resp {
            CoreResponse::Error { message } => {
                assert!(message.contains("No action pending"));
            }
            _ => panic!("Expected Error when no confirmation pending"),
        }
    }

    #[test]
    fn get_workspace_snapshot_returns_workspace() {
        let core = Core::new(make_storage());
        let ws = core.get_workspace_snapshot().unwrap();
        assert_eq!(ws.mode, WorkspaceMode::Idle);
    }

    #[test]
    fn get_recent_actions_empty_when_no_events() {
        let core = Core::new(make_storage());
        let actions = core.get_recent_actions(10).unwrap();
        assert!(actions.is_empty());
    }

    #[test]
    fn get_recent_actions_returns_safe_summaries() {
        let mut core = Core::new(make_storage());
        core.router_mut().register(calendar_metadata());

        core.submit_command("schedule a meeting").unwrap();
        core.submit_command("create an event").unwrap();

        let actions = core.get_recent_actions(10).unwrap();
        assert_eq!(actions.len(), 2);
        // Summaries include char count but never raw user text.
        assert_eq!(actions[0].description, "Command (18 chars)");
        assert_eq!(actions[1].description, "Command (15 chars)");
        // IDs are valid UUIDs.
        assert!(!actions[0].id.is_empty());
        assert_ne!(actions[0].id, actions[1].id);
    }

    #[test]
    fn get_recent_actions_respects_limit() {
        let mut core = Core::new(make_storage());
        core.router_mut().register(calendar_metadata());

        core.submit_command("first").unwrap();
        core.submit_command("second").unwrap();
        core.submit_command("third").unwrap();

        let actions = core.get_recent_actions(2).unwrap();
        assert_eq!(actions.len(), 2);
        // tail returns the last N; summaries distinguish by char count.
        assert_eq!(actions[0].description, "Command (6 chars)");
        assert_eq!(actions[1].description, "Command (5 chars)");
    }

    // --- Serde serialization tests (required by Core-12 test checklist) ---

    #[test]
    fn core_response_artifact_serializable() {
        let resp = CoreResponse::Artifact {
            content: "Routed to calendar".to_string(),
            actions: vec![ArtifactAction {
                id: "action-1".to_string(),
                label: "Confirm".to_string(),
            }],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Artifact"));
        assert!(json.contains("calendar"));
    }

    #[test]
    fn core_response_preview_serializable() {
        let resp = CoreResponse::Preview {
            title: "Last Note".to_string(),
            content: "Buy groceries".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Preview"));
        assert!(json.contains("Last Note"));
    }

    #[test]
    fn core_response_confirmation_serializable() {
        let resp = CoreResponse::Confirmation {
            confirmation_id: "confirm-12345".to_string(),
            prompt: "Delete this file?".to_string(),
            description: "This will permanently remove /tmp/test.txt".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Confirmation"));
        assert!(json.contains("confirm-12345"));
    }

    #[test]
    fn core_response_error_serializable() {
        let resp = CoreResponse::Error {
            message: "Something went wrong".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Error"));
        assert!(json.contains("Something went wrong"));
    }

    #[test]
    fn stub_command_produces_each_variant() {
        let mut core = Core::new(make_storage());
        core.router_mut().register(calendar_metadata());
        core.router_mut().register(notes_metadata());

        // Artifact: normal routing (non-preview verb).
        let resp = core.submit_command("schedule a meeting").unwrap();
        assert!(matches!(resp, CoreResponse::Artifact { .. }));

        // Preview: read-only verb triggers Preview when routing matches.
        let resp = core.submit_command("show last note").unwrap();
        assert!(
            matches!(resp, CoreResponse::Preview { .. }),
            "Expected Preview for read-only verb, got {:?}",
            resp
        );

        // Error: expired follow-up.
        core.workspace.follow_up = Some(FollowUpContext {
            last_command: "test".to_string(),
            last_result_entity_ids: vec![],
            last_app_id: "calendar".to_string(),
            expires_at: 0,
            turn_count: 0,
            max_turns: FOLLOW_UP_MAX_TURNS,
        });
        core.workspace.mode = WorkspaceMode::FollowUpActive;
        let resp = core.submit_command("update").unwrap();
        assert!(matches!(resp, CoreResponse::Error { .. }));

        // Confirmation: pending confirmation blocks new commands.
        core.workspace.confirmation_pending = Some(ConfirmationPending {
            confirmation_id: "c-1".to_string(),
            tool_id: "risky_tool".to_string(),
            args: serde_json::json!({}),
            requested_at: 100,
        });
        let resp = core.submit_command("do something").unwrap();
        assert!(
            matches!(resp, CoreResponse::Confirmation { .. }),
            "Expected Confirmation when pending, got {:?}",
            resp
        );
    }
}
