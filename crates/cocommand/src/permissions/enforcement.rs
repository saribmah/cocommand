//! Core permission enforcement logic.

use uuid::Uuid;

use crate::tools::ToolDefinition;
use super::risk::risk_for_tool;
use super::store::{PermissionDecision, PermissionStore};

/// Result of permission enforcement for a tool invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnforcementResult {
    /// The tool is allowed to execute.
    Allowed,
    /// The tool is denied execution.
    Denied { reason: String },
    /// The tool requires user confirmation before execution.
    NeedsConfirmation { confirmation_id: String },
}

/// Enforce permissions for a tool invocation.
///
/// Decision logic:
/// 1. Derive scope from the tool's `risk_level` via `risk_for_tool()`.
/// 2. Check the store for a `(tool_id, scope)` decision:
///    - `Allow` → `Allowed`
///    - `Deny` → `Denied`
///    - `Ask` → `NeedsConfirmation`
/// 3. No stored decision:
///    - Safe → `Allowed`
///    - Confirm/Destructive → `NeedsConfirmation`
///
/// Confirmation ID format: `"confirm-{uuid_first_8}-{tool_id}"` for deterministic replay.
pub fn enforce_permissions(
    tool: &ToolDefinition,
    store: &PermissionStore,
    tool_call_id: Uuid,
) -> EnforcementResult {
    let scope = risk_for_tool(&tool.risk_level);

    // Check stored decision
    if let Some(decision) = store.get_decision(&tool.tool_id, &scope) {
        return match decision {
            PermissionDecision::Allow => EnforcementResult::Allowed,
            PermissionDecision::Deny => EnforcementResult::Denied {
                reason: format!("tool '{}' denied by stored permission", tool.tool_id),
            },
            PermissionDecision::Ask => EnforcementResult::NeedsConfirmation {
                confirmation_id: make_confirmation_id(tool_call_id, &tool.tool_id),
            },
        };
    }

    // No stored decision: derive from risk level
    match tool.risk_level {
        crate::tools::RiskLevel::Safe => EnforcementResult::Allowed,
        crate::tools::RiskLevel::Confirm | crate::tools::RiskLevel::Destructive => {
            EnforcementResult::NeedsConfirmation {
                confirmation_id: make_confirmation_id(tool_call_id, &tool.tool_id),
            }
        }
    }
}

/// Build a deterministic confirmation ID from the tool call UUID and tool ID.
fn make_confirmation_id(tool_call_id: Uuid, tool_id: &str) -> String {
    let uuid_prefix = &tool_call_id.to_string()[..8];
    format!("confirm-{uuid_prefix}-{tool_id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::schema::{RiskLevel, ToolDefinition, ToolHandler};
    use crate::error::CoreResult;
    use serde_json::json;

    fn noop_handler() -> ToolHandler {
        Box::new(|_args, _ctx| -> CoreResult<serde_json::Value> { Ok(json!(null)) })
    }

    fn make_tool(id: &str, risk_level: RiskLevel) -> ToolDefinition {
        ToolDefinition {
            tool_id: id.to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            risk_level,
            is_kernel: false,
            handler: noop_handler(),
        }
    }

    #[test]
    fn safe_tool_empty_store_allowed() {
        let tool = make_tool("read_file", RiskLevel::Safe);
        let store = PermissionStore::new();
        let result = enforce_permissions(&tool, &store, Uuid::new_v4());
        assert_eq!(result, EnforcementResult::Allowed);
    }

    #[test]
    fn confirm_tool_empty_store_needs_confirmation() {
        let tool = make_tool("write_file", RiskLevel::Confirm);
        let store = PermissionStore::new();
        let result = enforce_permissions(&tool, &store, Uuid::new_v4());
        assert!(matches!(result, EnforcementResult::NeedsConfirmation { .. }));
    }

    #[test]
    fn destructive_tool_empty_store_needs_confirmation() {
        let tool = make_tool("delete_all", RiskLevel::Destructive);
        let store = PermissionStore::new();
        let result = enforce_permissions(&tool, &store, Uuid::new_v4());
        assert!(matches!(result, EnforcementResult::NeedsConfirmation { .. }));
    }

    #[test]
    fn stored_allow_overrides_risk() {
        let tool = make_tool("write_file", RiskLevel::Confirm);
        let mut store = PermissionStore::new();
        store.set_decision(
            "write_file".to_string(),
            super::super::scopes::PermissionScope::Write,
            PermissionDecision::Allow,
        );
        let result = enforce_permissions(&tool, &store, Uuid::new_v4());
        assert_eq!(result, EnforcementResult::Allowed);
    }

    #[test]
    fn stored_deny_returns_denied() {
        let tool = make_tool("write_file", RiskLevel::Confirm);
        let mut store = PermissionStore::new();
        store.set_decision(
            "write_file".to_string(),
            super::super::scopes::PermissionScope::Write,
            PermissionDecision::Deny,
        );
        let result = enforce_permissions(&tool, &store, Uuid::new_v4());
        assert!(matches!(result, EnforcementResult::Denied { .. }));
    }

    #[test]
    fn stored_ask_returns_needs_confirmation() {
        let tool = make_tool("read_file", RiskLevel::Safe);
        let mut store = PermissionStore::new();
        store.set_decision(
            "read_file".to_string(),
            super::super::scopes::PermissionScope::Read,
            PermissionDecision::Ask,
        );
        let result = enforce_permissions(&tool, &store, Uuid::new_v4());
        assert!(matches!(result, EnforcementResult::NeedsConfirmation { .. }));
    }

    #[test]
    fn confirmation_id_format() {
        let uuid = Uuid::parse_str("12345678-1234-1234-1234-123456789abc").unwrap();
        let tool = make_tool("my_tool", RiskLevel::Confirm);
        let store = PermissionStore::new();
        let result = enforce_permissions(&tool, &store, uuid);
        if let EnforcementResult::NeedsConfirmation { confirmation_id } = result {
            assert_eq!(confirmation_id, "confirm-12345678-my_tool");
        } else {
            panic!("expected NeedsConfirmation");
        }
    }

    #[test]
    fn confirmation_id_is_deterministic() {
        let uuid = Uuid::new_v4();
        let tool = make_tool("test_tool", RiskLevel::Destructive);
        let store = PermissionStore::new();
        let r1 = enforce_permissions(&tool, &store, uuid);
        let r2 = enforce_permissions(&tool, &store, uuid);
        assert_eq!(r1, r2);
    }
}
