//! Deterministic redaction of sensitive event fields.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

use super::event::Event;
use crate::tools::ToolInvocationRecord;
use crate::workspace::WorkspacePatch;

/// Placeholder used for redacted string content.
const REDACTED: &str = "[REDACTED]";

/// A redacted view of an event with sensitive fields replaced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RedactedEvent {
    /// A user message with redacted text.
    UserMessage {
        id: Uuid,
        timestamp: SystemTime,
        text: String,
    },
    /// A tool call proposal with redacted args.
    ToolCallProposed {
        id: Uuid,
        timestamp: SystemTime,
        tool_id: String,
        args: serde_json::Value,
    },
    /// A tool call authorization (no sensitive fields).
    ToolCallAuthorized {
        id: Uuid,
        timestamp: SystemTime,
        tool_call_id: Uuid,
    },
    /// A tool call denial with redacted reason.
    ToolCallDenied {
        id: Uuid,
        timestamp: SystemTime,
        tool_call_id: Uuid,
        reason: String,
    },
    /// A tool execution with redacted invocation details.
    ToolCallExecuted {
        id: Uuid,
        timestamp: SystemTime,
        tool_call_id: Uuid,
        invocation: ToolInvocationRecord,
    },
    /// A tool result with redacted value.
    ToolResultRecorded {
        id: Uuid,
        timestamp: SystemTime,
        tool_call_id: Uuid,
        result: serde_json::Value,
    },
    /// A workspace patch (structural, not redacted).
    WorkspacePatched {
        id: Uuid,
        timestamp: SystemTime,
        patch: WorkspacePatch,
        workspace_hash_before: String,
        workspace_hash_after: String,
    },
    /// An error with redacted message.
    ErrorRaised {
        id: Uuid,
        timestamp: SystemTime,
        code: String,
        message: String,
    },
}

/// Redact a single event, replacing sensitive fields with `[REDACTED]`.
///
/// The redaction is deterministic: the same input always produces the same output.
/// Sensitive fields include user message text, tool arguments, tool results,
/// denial reasons, and error messages.
pub fn redact_event(event: &Event) -> RedactedEvent {
    match event {
        Event::UserMessage { id, timestamp, .. } => RedactedEvent::UserMessage {
            id: *id,
            timestamp: *timestamp,
            text: REDACTED.to_string(),
        },
        Event::ToolCallProposed {
            id,
            timestamp,
            tool_id,
            ..
        } => RedactedEvent::ToolCallProposed {
            id: *id,
            timestamp: *timestamp,
            tool_id: tool_id.clone(),
            args: serde_json::Value::String(REDACTED.to_string()),
        },
        Event::ToolCallAuthorized {
            id,
            timestamp,
            tool_call_id,
        } => RedactedEvent::ToolCallAuthorized {
            id: *id,
            timestamp: *timestamp,
            tool_call_id: *tool_call_id,
        },
        Event::ToolCallDenied {
            id,
            timestamp,
            tool_call_id,
            ..
        } => RedactedEvent::ToolCallDenied {
            id: *id,
            timestamp: *timestamp,
            tool_call_id: *tool_call_id,
            reason: REDACTED.to_string(),
        },
        Event::ToolCallExecuted {
            id,
            timestamp,
            tool_call_id,
            invocation,
        } => {
            let mut redacted_invocation = invocation.clone();
            redacted_invocation.redaction_applied = true;
            RedactedEvent::ToolCallExecuted {
                id: *id,
                timestamp: *timestamp,
                tool_call_id: *tool_call_id,
                invocation: redacted_invocation,
            }
        }
        Event::ToolResultRecorded {
            id,
            timestamp,
            tool_call_id,
            ..
        } => RedactedEvent::ToolResultRecorded {
            id: *id,
            timestamp: *timestamp,
            tool_call_id: *tool_call_id,
            result: serde_json::Value::String(REDACTED.to_string()),
        },
        Event::WorkspacePatched {
            id,
            timestamp,
            patch,
            workspace_hash_before,
            workspace_hash_after,
        } => RedactedEvent::WorkspacePatched {
            id: *id,
            timestamp: *timestamp,
            patch: patch.clone(),
            workspace_hash_before: workspace_hash_before.clone(),
            workspace_hash_after: workspace_hash_after.clone(),
        },
        Event::ErrorRaised {
            id,
            timestamp,
            code,
            ..
        } => RedactedEvent::ErrorRaised {
            id: *id,
            timestamp: *timestamp,
            code: code.clone(),
            message: REDACTED.to_string(),
        },
    }
}

/// Redact a slice of events.
pub fn redact_events(events: &[Event]) -> Vec<RedactedEvent> {
    events.iter().map(redact_event).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::InvocationStatus;
    use std::time::Duration;

    #[test]
    fn redact_user_message_hides_text() {
        let event = Event::UserMessage {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            text: "secret user input".to_string(),
        };
        let redacted = redact_event(&event);
        match redacted {
            RedactedEvent::UserMessage { text, .. } => {
                assert_eq!(text, "[REDACTED]");
            }
            _ => panic!("expected UserMessage variant"),
        }
    }

    #[test]
    fn redact_tool_call_proposed_hides_args() {
        let event = Event::ToolCallProposed {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            tool_id: "my-tool".to_string(),
            args: serde_json::json!({"key": "sensitive-value"}),
        };
        let redacted = redact_event(&event);
        match redacted {
            RedactedEvent::ToolCallProposed {
                tool_id, args, ..
            } => {
                assert_eq!(tool_id, "my-tool");
                assert_eq!(args, serde_json::Value::String("[REDACTED]".to_string()));
            }
            _ => panic!("expected ToolCallProposed variant"),
        }
    }

    #[test]
    fn redact_tool_result_hides_result() {
        let event = Event::ToolResultRecorded {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            tool_call_id: Uuid::new_v4(),
            result: serde_json::json!({"data": "private"}),
        };
        let redacted = redact_event(&event);
        match redacted {
            RedactedEvent::ToolResultRecorded { result, .. } => {
                assert_eq!(result, serde_json::Value::String("[REDACTED]".to_string()));
            }
            _ => panic!("expected ToolResultRecorded variant"),
        }
    }

    #[test]
    fn redact_error_hides_message_preserves_code() {
        let event = Event::ErrorRaised {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            code: "E042".to_string(),
            message: "detailed internal error info".to_string(),
        };
        let redacted = redact_event(&event);
        match redacted {
            RedactedEvent::ErrorRaised { code, message, .. } => {
                assert_eq!(code, "E042");
                assert_eq!(message, "[REDACTED]");
            }
            _ => panic!("expected ErrorRaised variant"),
        }
    }

    #[test]
    fn redact_workspace_patched_is_not_redacted() {
        let event = Event::WorkspacePatched {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            patch: WorkspacePatch {
                operations: vec![],
            },
            workspace_hash_before: "abc".to_string(),
            workspace_hash_after: "def".to_string(),
        };
        let redacted = redact_event(&event);
        match redacted {
            RedactedEvent::WorkspacePatched {
                workspace_hash_before,
                workspace_hash_after,
                ..
            } => {
                assert_eq!(workspace_hash_before, "abc");
                assert_eq!(workspace_hash_after, "def");
            }
            _ => panic!("expected WorkspacePatched variant"),
        }
    }

    #[test]
    fn redact_tool_call_executed_marks_redaction() {
        let start = SystemTime::now();
        let end = start + Duration::from_millis(10);
        let invocation = ToolInvocationRecord::new(
            "tool-1".to_string(),
            start,
            end,
            InvocationStatus::Success,
            "h1".to_string(),
            "h2".to_string(),
        );
        let event = Event::ToolCallExecuted {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            tool_call_id: Uuid::new_v4(),
            invocation,
        };
        let redacted = redact_event(&event);
        match redacted {
            RedactedEvent::ToolCallExecuted { invocation, .. } => {
                assert!(invocation.redaction_applied);
            }
            _ => panic!("expected ToolCallExecuted variant"),
        }
    }

    #[test]
    fn redact_authorized_preserves_all_fields() {
        let id = Uuid::new_v4();
        let tool_call_id = Uuid::new_v4();
        let ts = SystemTime::now();
        let event = Event::ToolCallAuthorized {
            id,
            timestamp: ts,
            tool_call_id,
        };
        let redacted = redact_event(&event);
        match redacted {
            RedactedEvent::ToolCallAuthorized {
                id: rid,
                tool_call_id: rtcid,
                ..
            } => {
                assert_eq!(rid, id);
                assert_eq!(rtcid, tool_call_id);
            }
            _ => panic!("expected ToolCallAuthorized variant"),
        }
    }

    #[test]
    fn redact_events_batch() {
        let events = vec![
            Event::UserMessage {
                id: Uuid::new_v4(),
                timestamp: SystemTime::now(),
                text: "msg1".to_string(),
            },
            Event::ErrorRaised {
                id: Uuid::new_v4(),
                timestamp: SystemTime::now(),
                code: "E1".to_string(),
                message: "secret".to_string(),
            },
        ];
        let redacted = redact_events(&events);
        assert_eq!(redacted.len(), 2);
    }

    #[test]
    fn redaction_is_deterministic() {
        let event = Event::UserMessage {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            text: "same input".to_string(),
        };
        let r1 = serde_json::to_string(&redact_event(&event)).unwrap();
        let r2 = serde_json::to_string(&redact_event(&event)).unwrap();
        assert_eq!(r1, r2);
    }
}
