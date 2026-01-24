//! Domain structs for the storage layer.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

use crate::events::Event;

/// A stored event record with a sequence number for deterministic ordering.
///
/// The `summary` field is a pre-computed, persistence-safe display string
/// derived from structural event metadata (tool IDs, error codes, message
/// length). It never contains raw user text or sensitive content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub seq: u64,
    pub event: Event,
    /// Human-readable summary safe for UI display and persistence.
    pub summary: String,
}

impl EventRecord {
    pub fn id(&self) -> Uuid {
        self.event.id()
    }

    pub fn timestamp(&self) -> SystemTime {
        self.event.timestamp()
    }
}

/// Compute a persistence-safe summary for an event.
///
/// Uses only structural metadata (tool IDs, error codes, text length) —
/// never includes raw user input, tool arguments, or error messages.
pub fn event_summary(event: &Event) -> String {
    match event {
        Event::UserMessage { text, .. } => {
            format!("Command ({} chars)", text.len())
        }
        Event::ToolCallProposed { tool_id, .. } => {
            format!("Proposed: {}", tool_id)
        }
        Event::ToolCallAuthorized { .. } => "Authorized tool call".to_string(),
        Event::ToolCallDenied { .. } => "Denied tool call".to_string(),
        Event::ToolCallExecuted { invocation, .. } => {
            format!("Executed: {}", invocation.tool_id)
        }
        Event::ToolResultRecorded { .. } => "Result recorded".to_string(),
        Event::WorkspacePatched { .. } => "Workspace updated".to_string(),
        Event::ErrorRaised { code, .. } => {
            format!("Error ({})", code)
        }
    }
}

/// A serializable workspace snapshot for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub session_id: String,
    pub captured_at: SystemTime,
    pub data: serde_json::Value,
}

/// A clipboard history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub id: Uuid,
    pub content: String,
    pub copied_at: SystemTime,
}
