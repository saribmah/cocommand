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

/// Context for a follow-up conversation turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowUpContext {
    pub last_command: String,
    pub last_result_entity_ids: Vec<String>,
    pub expires_at: Timestamp,
    pub turn_count: usize,
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
