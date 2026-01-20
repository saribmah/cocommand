//! Application and tool type definitions.
//!
//! This module contains the core traits and data types for applications and tools.

use serde::Serialize;
use serde_json::Value;

/// Trait for applications that provide tools to the agent.
///
/// Each application has an identifier, display name, description,
/// and a list of tools it provides when opened.
pub trait Application {
    /// Unique identifier for the application (e.g., "spotify").
    fn id(&self) -> &str;

    /// Display name (e.g., "Spotify").
    fn name(&self) -> &str;

    /// Brief description of the application's capabilities.
    fn description(&self) -> &str;

    /// List of tools this application provides when open.
    fn tools(&self) -> Vec<ToolDefinition>;
}

/// Trait for executable tools.
///
/// Tools are the atomic units of functionality that the agent can invoke.
pub trait Tool {
    /// Unique identifier for the tool (e.g., "spotify.play").
    fn id(&self) -> &str;

    /// Display name (e.g., "Play").
    fn name(&self) -> &str;

    /// Brief description of what the tool does.
    fn description(&self) -> &str;

    /// Execute the tool with the given inputs.
    fn execute(&self, inputs: Value) -> ToolResult;
}

/// Result of a tool execution.
#[derive(Clone, Serialize, Debug)]
pub struct ToolResult {
    /// Status of the execution ("ok" or "error").
    pub status: String,
    /// Human-readable message describing the result.
    pub message: String,
}

impl ToolResult {
    /// Create a successful result.
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            status: "ok".to_string(),
            message: message.into(),
        }
    }

    /// Create an error result.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            message: message.into(),
        }
    }
}

/// Static definition of a tool (used for listing/discovery).
#[derive(Clone, Serialize, Debug)]
pub struct ToolDefinition {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Description of the tool.
    pub description: String,
}

/// Static definition of an application (used for listing/discovery).
#[derive(Clone, Serialize, Debug)]
pub struct ApplicationDefinition {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Description of the application.
    pub description: String,
    /// List of tools this application provides.
    pub tools: Vec<ToolDefinition>,
}

/// Helper to create a ToolDefinition from a Tool trait object.
pub fn tool_definition<T: Tool>(tool: &T) -> ToolDefinition {
    ToolDefinition {
        id: tool.id().to_string(),
        name: tool.name().to_string(),
        description: tool.description().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_result_ok() {
        let result = ToolResult::ok("Success");
        assert_eq!(result.status, "ok");
        assert_eq!(result.message, "Success");
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("Failed");
        assert_eq!(result.status, "error");
        assert_eq!(result.message, "Failed");
    }
}
