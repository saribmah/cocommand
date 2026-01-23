//! Canonical event types for the cocommand event stream.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

use crate::tools::ToolInvocationRecord;
use crate::workspace::WorkspacePatch;

/// A canonical event in the cocommand event stream.
///
/// Each event has a unique ID and timestamp. Events are append-only and
/// represent the full lifecycle of actions within the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    /// A user message was received.
    UserMessage {
        id: Uuid,
        timestamp: SystemTime,
        text: String,
    },
    /// A tool call was proposed by the planner.
    ToolCallProposed {
        id: Uuid,
        timestamp: SystemTime,
        tool_id: String,
        args: serde_json::Value,
    },
    /// A proposed tool call was authorized for execution.
    ToolCallAuthorized {
        id: Uuid,
        timestamp: SystemTime,
        tool_call_id: Uuid,
    },
    /// A proposed tool call was denied.
    ToolCallDenied {
        id: Uuid,
        timestamp: SystemTime,
        tool_call_id: Uuid,
        reason: String,
    },
    /// A tool call was executed and produced an invocation record.
    ToolCallExecuted {
        id: Uuid,
        timestamp: SystemTime,
        tool_call_id: Uuid,
        invocation: ToolInvocationRecord,
    },
    /// A tool call produced a result value.
    ToolResultRecorded {
        id: Uuid,
        timestamp: SystemTime,
        tool_call_id: Uuid,
        result: serde_json::Value,
    },
    /// A workspace patch was applied.
    WorkspacePatched {
        id: Uuid,
        timestamp: SystemTime,
        patch: WorkspacePatch,
        workspace_hash_before: String,
        workspace_hash_after: String,
    },
    /// An error was raised during processing.
    ErrorRaised {
        id: Uuid,
        timestamp: SystemTime,
        code: String,
        message: String,
    },
}

impl Event {
    /// Returns the unique ID of this event.
    pub fn id(&self) -> Uuid {
        match self {
            Event::UserMessage { id, .. }
            | Event::ToolCallProposed { id, .. }
            | Event::ToolCallAuthorized { id, .. }
            | Event::ToolCallDenied { id, .. }
            | Event::ToolCallExecuted { id, .. }
            | Event::ToolResultRecorded { id, .. }
            | Event::WorkspacePatched { id, .. }
            | Event::ErrorRaised { id, .. } => *id,
        }
    }

    /// Returns the timestamp of this event.
    pub fn timestamp(&self) -> SystemTime {
        match self {
            Event::UserMessage { timestamp, .. }
            | Event::ToolCallProposed { timestamp, .. }
            | Event::ToolCallAuthorized { timestamp, .. }
            | Event::ToolCallDenied { timestamp, .. }
            | Event::ToolCallExecuted { timestamp, .. }
            | Event::ToolResultRecorded { timestamp, .. }
            | Event::WorkspacePatched { timestamp, .. }
            | Event::ErrorRaised { timestamp, .. } => *timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_event_accessors() {
        let id = Uuid::new_v4();
        let ts = SystemTime::now();
        let event = Event::UserMessage {
            id,
            timestamp: ts,
            text: "hello".to_string(),
        };
        assert_eq!(event.id(), id);
        assert_eq!(event.timestamp(), ts);
    }

    #[test]
    fn error_raised_event_accessors() {
        let id = Uuid::new_v4();
        let ts = SystemTime::now();
        let event = Event::ErrorRaised {
            id,
            timestamp: ts,
            code: "E001".to_string(),
            message: "something failed".to_string(),
        };
        assert_eq!(event.id(), id);
        assert_eq!(event.timestamp(), ts);
    }

    #[test]
    fn event_serialize_roundtrip() {
        let event = Event::UserMessage {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            text: "test message".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(event.id(), deserialized.id());
    }
}
