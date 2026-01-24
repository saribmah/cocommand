//! Tool invocation records capturing execution metadata.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Status of a tool invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvocationStatus {
    /// The tool executed successfully.
    Success,
    /// The tool execution failed.
    Failed,
    /// The tool invocation was denied by the permission system.
    Denied,
}

/// Record of a single tool invocation, capturing timing, status, and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocationRecord {
    /// Identifier of the tool that was invoked.
    pub tool_id: String,
    /// When the invocation started.
    pub started_at: SystemTime,
    /// When the invocation ended.
    pub ended_at: SystemTime,
    /// Duration of the invocation in milliseconds.
    pub duration_ms: u64,
    /// Outcome of the invocation.
    pub status: InvocationStatus,
    /// Error code if the invocation failed.
    pub error_code: Option<String>,
    /// Whether redaction was applied to the invocation output.
    pub redaction_applied: bool,
    /// Hash of the workspace state before the invocation.
    pub workspace_hash_before: String,
    /// Hash of the workspace state after the invocation.
    pub workspace_hash_after: String,
    /// Model ID associated with this invocation (v0 placeholder).
    pub model_id: Option<String>,
    /// Prompt version associated with this invocation (v0 placeholder).
    pub prompt_version: Option<String>,
}

impl ToolInvocationRecord {
    /// Create a new invocation record from start/end times.
    ///
    /// Computes `duration_ms` from the difference between `ended_at` and `started_at`.
    pub fn new(
        tool_id: String,
        started_at: SystemTime,
        ended_at: SystemTime,
        status: InvocationStatus,
        workspace_hash_before: String,
        workspace_hash_after: String,
    ) -> Self {
        let duration_ms = ended_at
            .duration_since(started_at)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            tool_id,
            started_at,
            ended_at,
            duration_ms,
            status,
            error_code: None,
            redaction_applied: false,
            workspace_hash_before,
            workspace_hash_after,
            model_id: None,
            prompt_version: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn new_computes_duration() {
        let start = SystemTime::now();
        let end = start + Duration::from_millis(150);
        let record = ToolInvocationRecord::new(
            "test-tool".to_string(),
            start,
            end,
            InvocationStatus::Success,
            "hash-before".to_string(),
            "hash-after".to_string(),
        );
        assert_eq!(record.duration_ms, 150);
        assert_eq!(record.tool_id, "test-tool");
        assert_eq!(record.status, InvocationStatus::Success);
        assert!(!record.redaction_applied);
        assert_eq!(record.error_code, None);
    }

    #[test]
    fn failed_invocation_with_error_code() {
        let start = SystemTime::now();
        let end = start + Duration::from_millis(50);
        let mut record = ToolInvocationRecord::new(
            "failing-tool".to_string(),
            start,
            end,
            InvocationStatus::Failed,
            "h1".to_string(),
            "h1".to_string(),
        );
        record.error_code = Some("TIMEOUT".to_string());
        assert_eq!(record.status, InvocationStatus::Failed);
        assert_eq!(record.error_code, Some("TIMEOUT".to_string()));
    }

    #[test]
    fn serialize_roundtrip() {
        let start = SystemTime::now();
        let end = start + Duration::from_millis(100);
        let record = ToolInvocationRecord::new(
            "tool-x".to_string(),
            start,
            end,
            InvocationStatus::Denied,
            "before".to_string(),
            "before".to_string(),
        );
        let json = serde_json::to_string(&record).unwrap();
        let deserialized: ToolInvocationRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tool_id, "tool-x");
        assert_eq!(deserialized.status, InvocationStatus::Denied);
        assert_eq!(deserialized.duration_ms, 100);
    }
}
