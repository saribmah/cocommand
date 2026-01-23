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

    /// JSON schema for the tool's parameters (optional).
    /// Returns None if the tool takes no parameters.
    fn schema(&self) -> Option<Value> {
        None
    }

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
    /// JSON schema for tool parameters (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<Value>,
}

impl ToolDefinition {
    /// Create a tool definition with no parameters.
    pub fn no_params(id: impl Into<String>, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            schema: None,
        }
    }

    /// Create a tool definition with a JSON schema.
    pub fn with_schema(id: impl Into<String>, name: impl Into<String>, description: impl Into<String>, schema: Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            schema: Some(schema),
        }
    }

    /// Get the schema as JSON, defaulting to empty object if none specified.
    pub fn schema_json(&self) -> Value {
        self.schema.clone().unwrap_or_else(|| serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        }))
    }
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
/// Uses the tool's schema() method if available.
pub fn tool_definition<T: Tool>(tool: &T) -> ToolDefinition {
    match tool.schema() {
        Some(schema) => ToolDefinition::with_schema(tool.id(), tool.name(), tool.description(), schema),
        None => ToolDefinition::no_params(tool.id(), tool.name(), tool.description()),
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
