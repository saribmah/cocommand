//! JSON-RPC protocol types for stdio communication with the Deno extension host.
//!
//! The Rust core sends requests to the Deno host via stdin and reads responses
//! from stdout, using newline-delimited JSON-RPC 2.0 messages.

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request sent from Rust core to Deno host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    /// Protocol version (always "2.0").
    pub jsonrpc: String,
    /// Request identifier for correlating responses.
    pub id: u64,
    /// Method name (e.g., "initialize", "invoke_tool").
    pub method: String,
    /// Method parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 response received from Deno host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    /// Protocol version (always "2.0").
    pub jsonrpc: String,
    /// Matching request identifier.
    pub id: u64,
    /// Successful result (mutually exclusive with `error`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error object (mutually exclusive with `result`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    /// Error code.
    pub code: i64,
    /// Human-readable error message.
    pub message: String,
    /// Optional additional data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Standard JSON-RPC error codes.
pub mod error_codes {
    /// Parse error: invalid JSON.
    pub const PARSE_ERROR: i64 = -32700;
    /// Invalid request: missing required fields.
    pub const INVALID_REQUEST: i64 = -32600;
    /// Method not found.
    pub const METHOD_NOT_FOUND: i64 = -32601;
    /// Invalid params.
    pub const INVALID_PARAMS: i64 = -32602;
    /// Internal error.
    pub const INTERNAL_ERROR: i64 = -32603;
    /// Tool execution failed (application-defined).
    pub const TOOL_EXECUTION_ERROR: i64 = -32000;
}

/// Parameters for the `initialize` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    /// Path to the extension root directory.
    pub extension_dir: String,
    /// Extension ID from manifest.
    pub extension_id: String,
}

/// Result of the `initialize` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    /// List of tool IDs successfully registered by the host.
    pub tools: Vec<String>,
}

/// Parameters for the `invoke_tool` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeToolParams {
    /// Tool identifier to invoke.
    pub tool_id: String,
    /// Arguments passed to the tool handler.
    pub args: serde_json::Value,
}

/// Result of the `invoke_tool` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeToolResult {
    /// Tool output as JSON.
    pub output: serde_json::Value,
}

impl RpcRequest {
    /// Create a new JSON-RPC 2.0 request.
    pub fn new(id: u64, method: &str, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        }
    }
}

impl RpcResponse {
    /// Check if this response indicates an error.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Create a successful response.
    pub fn success(id: u64, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(id: u64, code: i64, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(RpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_serializes_correctly() {
        let req = RpcRequest::new(
            1,
            "invoke_tool",
            Some(json!({"tool_id": "my_app.create_ticket", "args": {"title": "Bug"}})),
        );
        let serialized = serde_json::to_string(&req).unwrap();
        assert!(serialized.contains("\"jsonrpc\":\"2.0\""));
        assert!(serialized.contains("\"method\":\"invoke_tool\""));
        assert!(serialized.contains("\"id\":1"));
    }

    #[test]
    fn request_without_params_omits_field() {
        let req = RpcRequest::new(1, "shutdown", None);
        let serialized = serde_json::to_string(&req).unwrap();
        assert!(!serialized.contains("params"));
    }

    #[test]
    fn success_response_roundtrip() {
        let resp = RpcResponse::success(1, json!({"ticket_id": "T-123"}));
        let json_str = serde_json::to_string(&resp).unwrap();
        let parsed: RpcResponse = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.id, 1);
        assert!(!parsed.is_error());
        assert_eq!(parsed.result.unwrap()["ticket_id"], "T-123");
    }

    #[test]
    fn error_response_roundtrip() {
        let resp = RpcResponse::error(2, error_codes::METHOD_NOT_FOUND, "unknown method");
        let json_str = serde_json::to_string(&resp).unwrap();
        let parsed: RpcResponse = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.id, 2);
        assert!(parsed.is_error());
        let err = parsed.error.unwrap();
        assert_eq!(err.code, error_codes::METHOD_NOT_FOUND);
        assert_eq!(err.message, "unknown method");
    }

    #[test]
    fn initialize_params_serde() {
        let params = InitializeParams {
            extension_dir: "/path/to/ext".to_string(),
            extension_id: "my-app".to_string(),
        };
        let value = serde_json::to_value(&params).unwrap();
        let parsed: InitializeParams = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.extension_id, "my-app");
    }

    #[test]
    fn invoke_tool_params_serde() {
        let params = InvokeToolParams {
            tool_id: "my_app.create_ticket".to_string(),
            args: json!({"title": "Fix bug"}),
        };
        let value = serde_json::to_value(&params).unwrap();
        let parsed: InvokeToolParams = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.tool_id, "my_app.create_ticket");
        assert_eq!(parsed.args["title"], "Fix bug");
    }

    #[test]
    fn invoke_tool_result_serde() {
        let result = InvokeToolResult {
            output: json!({"ticket_id": "T-456", "status": "created"}),
        };
        let value = serde_json::to_value(&result).unwrap();
        let parsed: InvokeToolResult = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.output["ticket_id"], "T-456");
    }
}
