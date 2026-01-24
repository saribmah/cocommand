use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for an application instance.
pub type InstanceId = String;

/// Unique identifier for a tool.
pub type ToolId = String;

/// Unix timestamp in seconds.
pub type Timestamp = u64;

/// Whether an application instance is active or inactive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApplicationStatus {
    Active,
    Inactive,
}

/// Current mode of the workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceMode {
    Idle,
    FollowUpActive,
    AwaitingConfirmation,
}

/// A running instance of an application in the workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationInstance {
    pub instance_id: InstanceId,
    pub app_id: String,
    pub status: ApplicationStatus,
    pub context: HashMap<String, serde_json::Value>,
    pub mounted_tools: Vec<ToolId>,
}

/// Default TTL for follow-up mode in seconds (within spec range 60–120s).
pub const FOLLOW_UP_TTL_SECS: u64 = 90;

/// Maximum number of follow-up turns before context expires.
pub const FOLLOW_UP_MAX_TURNS: usize = 3;

/// Context for a follow-up conversation turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowUpContext {
    /// The command that initiated this follow-up window.
    pub last_command: String,
    /// Entity IDs produced or modified by the last command.
    pub last_result_entity_ids: Vec<String>,
    /// The app that handled the last command (used for router bias).
    pub last_app_id: String,
    /// Unix timestamp at which this follow-up expires.
    pub expires_at: Timestamp,
    /// Number of follow-up turns consumed so far.
    pub turn_count: usize,
    /// Maximum turns allowed in this follow-up window.
    pub max_turns: usize,
}

/// A pending confirmation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationPending {
    pub confirmation_id: String,
    pub tool_id: String,
    pub args: serde_json::Value,
    pub requested_at: Timestamp,
}

/// The workspace state, containing all application instances and session metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub instances: HashMap<InstanceId, ApplicationInstance>,
    pub focus: Option<InstanceId>,
    pub mode: WorkspaceMode,
    pub follow_up: Option<FollowUpContext>,
    pub confirmation_pending: Option<ConfirmationPending>,
    pub session_id: String,
    pub created_at: Timestamp,
    pub last_modified: Timestamp,
}

impl Workspace {
    /// Create a new workspace with the given session ID.
    pub fn new(session_id: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            instances: HashMap::new(),
            focus: None,
            mode: WorkspaceMode::Idle,
            follow_up: None,
            confirmation_pending: None,
            session_id,
            created_at: now,
            last_modified: now,
        }
    }

    /// Generate a new unique instance ID.
    pub fn new_instance_id() -> InstanceId {
        Uuid::new_v4().to_string()
    }

    /// Clear the pending confirmation and reset workspace mode to Idle.
    pub fn clear_confirmation(&mut self) {
        self.confirmation_pending = None;
        self.mode = WorkspaceMode::Idle;
    }

    /// Enter follow-up mode after a successful command.
    ///
    /// Records the entity references, sets TTL, and transitions mode.
    pub fn enter_follow_up(
        &mut self,
        command: String,
        entity_ids: Vec<String>,
        app_id: String,
    ) {
        let now = Self::now();
        self.follow_up = Some(FollowUpContext {
            last_command: command,
            last_result_entity_ids: entity_ids,
            last_app_id: app_id,
            expires_at: now + FOLLOW_UP_TTL_SECS,
            turn_count: 0,
            max_turns: FOLLOW_UP_MAX_TURNS,
        });
        self.mode = WorkspaceMode::FollowUpActive;
        self.last_modified = now;
    }

    /// Check whether follow-up context is still valid at the given timestamp.
    pub fn is_follow_up_valid(&self, now: Timestamp) -> bool {
        match &self.follow_up {
            Some(ctx) => now < ctx.expires_at && ctx.turn_count < ctx.max_turns,
            None => false,
        }
    }

    /// Consume one follow-up turn. Returns `true` if the turn was consumed
    /// (context still valid), `false` if expired or absent.
    pub fn consume_follow_up_turn(&mut self) -> bool {
        let now = Self::now();
        if !self.is_follow_up_valid(now) {
            self.expire_follow_up();
            return false;
        }
        if let Some(ref mut ctx) = self.follow_up {
            ctx.turn_count += 1;
            self.last_modified = now;
            // If we've hit the limit, expire immediately.
            if ctx.turn_count >= ctx.max_turns {
                self.expire_follow_up();
                return false;
            }
        }
        true
    }

    /// Expire follow-up context and transition back to Idle.
    pub fn expire_follow_up(&mut self) {
        self.follow_up = None;
        self.mode = WorkspaceMode::Idle;
        self.last_modified = Self::now();
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

    #[test]
    fn workspace_creation_defaults() {
        let ws = Workspace::new("test-session".to_string());
        assert_eq!(ws.session_id, "test-session");
        assert!(ws.instances.is_empty());
        assert_eq!(ws.focus, None);
        assert_eq!(ws.mode, WorkspaceMode::Idle);
        assert!(ws.follow_up.is_none());
        assert!(ws.confirmation_pending.is_none());
        assert!(ws.created_at > 0);
        assert_eq!(ws.created_at, ws.last_modified);
    }

    #[test]
    fn clear_confirmation_resets_state() {
        let mut ws = Workspace::new("test-session".to_string());
        ws.mode = WorkspaceMode::AwaitingConfirmation;
        ws.confirmation_pending = Some(ConfirmationPending {
            confirmation_id: "confirm-12345678-tool".to_string(),
            tool_id: "test_tool".to_string(),
            args: serde_json::json!({}),
            requested_at: 1000,
        });

        ws.clear_confirmation();

        assert_eq!(ws.mode, WorkspaceMode::Idle);
        assert!(ws.confirmation_pending.is_none());
    }

    #[test]
    fn enter_follow_up_transitions_mode() {
        let mut ws = Workspace::new("test-session".to_string());
        assert_eq!(ws.mode, WorkspaceMode::Idle);

        ws.enter_follow_up(
            "create event".to_string(),
            vec!["evt-1".to_string()],
            "calendar".to_string(),
        );

        assert_eq!(ws.mode, WorkspaceMode::FollowUpActive);
        let ctx = ws.follow_up.as_ref().unwrap();
        assert_eq!(ctx.last_command, "create event");
        assert_eq!(ctx.last_result_entity_ids, vec!["evt-1".to_string()]);
        assert_eq!(ctx.last_app_id, "calendar");
        assert_eq!(ctx.turn_count, 0);
        assert_eq!(ctx.max_turns, FOLLOW_UP_MAX_TURNS);
        assert!(ctx.expires_at > 0);
    }

    #[test]
    fn is_follow_up_valid_respects_ttl() {
        let mut ws = Workspace::new("test-session".to_string());
        ws.enter_follow_up(
            "cmd".to_string(),
            vec!["id-1".to_string()],
            "app".to_string(),
        );

        let ctx = ws.follow_up.as_ref().unwrap();
        let before_expiry = ctx.expires_at - 1;
        let after_expiry = ctx.expires_at + 1;

        assert!(ws.is_follow_up_valid(before_expiry));
        assert!(!ws.is_follow_up_valid(after_expiry));
    }

    #[test]
    fn is_follow_up_valid_respects_turn_limit() {
        let mut ws = Workspace::new("test-session".to_string());
        ws.enter_follow_up(
            "cmd".to_string(),
            vec!["id-1".to_string()],
            "app".to_string(),
        );

        // Exhaust turns.
        if let Some(ref mut ctx) = ws.follow_up {
            ctx.turn_count = ctx.max_turns;
        }

        let now = ws.follow_up.as_ref().unwrap().expires_at - 10;
        assert!(!ws.is_follow_up_valid(now));
    }

    #[test]
    fn consume_follow_up_turn_increments() {
        let mut ws = Workspace::new("test-session".to_string());
        ws.enter_follow_up(
            "cmd".to_string(),
            vec!["id-1".to_string()],
            "app".to_string(),
        );

        assert!(ws.consume_follow_up_turn());
        assert_eq!(ws.follow_up.as_ref().unwrap().turn_count, 1);
    }

    #[test]
    fn consume_follow_up_turn_expires_at_max() {
        let mut ws = Workspace::new("test-session".to_string());
        ws.enter_follow_up(
            "cmd".to_string(),
            vec!["id-1".to_string()],
            "app".to_string(),
        );

        // Set to one turn before max.
        if let Some(ref mut ctx) = ws.follow_up {
            ctx.turn_count = ctx.max_turns - 1;
        }

        // This should be the last turn — triggers expiration.
        let result = ws.consume_follow_up_turn();
        assert!(!result);
        assert_eq!(ws.mode, WorkspaceMode::Idle);
        assert!(ws.follow_up.is_none());
    }

    #[test]
    fn expire_follow_up_clears_state() {
        let mut ws = Workspace::new("test-session".to_string());
        ws.enter_follow_up(
            "cmd".to_string(),
            vec!["id-1".to_string()],
            "app".to_string(),
        );

        ws.expire_follow_up();

        assert_eq!(ws.mode, WorkspaceMode::Idle);
        assert!(ws.follow_up.is_none());
    }

    #[test]
    fn follow_up_context_serializes() {
        let mut ws = Workspace::new("test-session".to_string());
        ws.enter_follow_up(
            "create event at 2pm".to_string(),
            vec!["evt-1".to_string(), "evt-2".to_string()],
            "calendar".to_string(),
        );

        let json = serde_json::to_string(&ws).expect("serialize");
        let deserialized: Workspace = serde_json::from_str(&json).expect("deserialize");

        let ctx = deserialized.follow_up.unwrap();
        assert_eq!(ctx.last_command, "create event at 2pm");
        assert_eq!(ctx.last_result_entity_ids, vec!["evt-1", "evt-2"]);
        assert_eq!(ctx.last_app_id, "calendar");
        assert_eq!(ctx.max_turns, FOLLOW_UP_MAX_TURNS);
    }

    #[test]
    fn workspace_serialize_deserialize_roundtrip() {
        let mut ws = Workspace::new("roundtrip-session".to_string());
        let instance = ApplicationInstance {
            instance_id: "inst-1".to_string(),
            app_id: "test-app".to_string(),
            status: ApplicationStatus::Active,
            context: HashMap::new(),
            mounted_tools: vec!["tool-a".to_string()],
        };
        ws.instances.insert("inst-1".to_string(), instance);
        ws.focus = Some("inst-1".to_string());

        let json = serde_json::to_string(&ws).expect("serialize");
        let deserialized: Workspace = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.session_id, ws.session_id);
        assert_eq!(deserialized.focus, Some("inst-1".to_string()));
        assert!(deserialized.instances.contains_key("inst-1"));
        let inst = &deserialized.instances["inst-1"];
        assert_eq!(inst.app_id, "test-app");
        assert_eq!(inst.status, ApplicationStatus::Active);
        assert_eq!(inst.mounted_tools, vec!["tool-a".to_string()]);
    }
}
