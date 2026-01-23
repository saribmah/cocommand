//! Shared types for API routes.
//!
//! This module contains request and response types used across different
//! route handlers.

use serde::{Deserialize, Serialize};

use crate::applications;
use crate::commands::intake as command_intake;
use crate::workspace::types::WorkspaceSnapshot;

// ============================================================================
// Command Types
// ============================================================================

/// Request body for the /command endpoint.
#[derive(Deserialize)]
pub struct CommandSubmitRequest {
    pub text: String,
}

/// Response for the /command endpoint.
#[derive(Serialize)]
pub struct CommandSubmitResponse {
    pub status: String,
    pub command: Option<command_intake::CommandInput>,
    pub app_id: Option<String>,
    pub tool_id: Option<String>,
    pub result: Option<applications::ToolResult>,
    pub message: Option<String>,
    /// The phase of execution that produced the result (control or execution)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    /// Number of turns used in the agent loop
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turns_used: Option<u32>,
}

impl CommandSubmitResponse {
    /// Create an empty command response.
    pub fn empty(message: impl Into<String>) -> Self {
        Self {
            status: "empty".to_string(),
            command: None,
            app_id: None,
            tool_id: None,
            result: None,
            message: Some(message.into()),
            phase: None,
            turns_used: None,
        }
    }

    /// Create a success response.
    pub fn success(
        command: command_intake::CommandInput,
        output: String,
        phase: &str,
        turns_used: u32,
    ) -> Self {
        Self {
            status: "ok".to_string(),
            command: Some(command),
            app_id: None,
            tool_id: None,
            result: Some(applications::ToolResult {
                status: "ok".to_string(),
                message: output,
            }),
            message: None,
            phase: Some(phase.to_string()),
            turns_used: Some(turns_used),
        }
    }

    /// Create an error response.
    pub fn error(command: command_intake::CommandInput, message: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            command: Some(command),
            app_id: None,
            tool_id: None,
            result: None,
            message: Some(message.into()),
            phase: None,
            turns_used: None,
        }
    }
}

// ============================================================================
// Execute Types
// ============================================================================

/// Request body for the /execute endpoint.
#[derive(Deserialize)]
pub struct ExecuteRequest {
    pub tool_id: String,
    pub inputs: serde_json::Value,
}

/// Response for the /execute endpoint.
#[derive(Serialize)]
pub struct ExecuteResponse {
    pub status: String,
    pub message: Option<String>,
}

impl ExecuteResponse {
    /// Create a success response.
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            status: "ok".to_string(),
            message: Some(message.into()),
        }
    }

    /// Create an error response.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            message: Some(message.into()),
        }
    }
}

// ============================================================================
// Window Types
// ============================================================================

/// Request body for window operations.
#[derive(Deserialize)]
pub struct WindowAppRequest {
    #[serde(rename = "appId")]
    pub app_id: String,
}

/// Response for window operations.
#[derive(Serialize)]
pub struct WindowResponse {
    pub status: String,
    pub snapshot: Option<WorkspaceSnapshot>,
    pub message: Option<String>,
    /// Whether the workspace was soft-reset due to inactivity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub soft_reset: Option<bool>,
    /// Whether the workspace is archived and requires restore
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
}

impl WindowResponse {
    /// Create a success response with snapshot.
    pub fn success(snapshot: WorkspaceSnapshot) -> Self {
        Self {
            status: "ok".to_string(),
            snapshot: Some(snapshot),
            message: None,
            soft_reset: None,
            archived: None,
        }
    }

    /// Create an error response.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            snapshot: None,
            message: Some(message.into()),
            soft_reset: None,
            archived: None,
        }
    }

    /// Create an archived error response.
    pub fn archived() -> Self {
        Self {
            status: "error".to_string(),
            snapshot: None,
            message: Some(
                "Workspace is archived. Use window.restore_workspace to recover.".to_string(),
            ),
            soft_reset: None,
            archived: Some(true),
        }
    }

    /// Create a response with lifecycle information.
    pub fn with_lifecycle(
        snapshot: WorkspaceSnapshot,
        lifecycle_message: Option<String>,
        is_soft_reset: bool,
        is_archived: bool,
    ) -> Self {
        Self {
            status: "ok".to_string(),
            snapshot: Some(snapshot),
            message: lifecycle_message,
            soft_reset: if is_soft_reset { Some(true) } else { None },
            archived: if is_archived { Some(true) } else { None },
        }
    }
}
