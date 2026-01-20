//! Tool-related types for the session.
//!
//! This module contains the types for tool calls and tool results
//! that are used in agent conversations.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents a tool call made by the assistant.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// Represents the result of a tool execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: Value,
    pub is_error: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_call_serialization() {
        let call = ToolCall {
            id: "call_123".to_string(),
            name: "window_open".to_string(),
            arguments: json!({"appId": "spotify"}),
        };

        let json = serde_json::to_string(&call).unwrap();
        assert!(json.contains("window_open"));
    }

    #[test]
    fn test_tool_result_serialization() {
        let result = ToolResult {
            tool_call_id: "call_123".to_string(),
            content: json!({"status": "ok"}),
            is_error: false,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("call_123"));
    }
}
