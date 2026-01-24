//! Tool execution pipeline.

use std::time::SystemTime;
use uuid::Uuid;

use crate::events::event::Event;
use crate::permissions::{enforce_permissions, EnforcementResult, PermissionStore};
use crate::storage::{ClipboardStore, EventLog};
use crate::workspace::{ConfirmationPending, Workspace, WorkspaceMode, WorkspacePatch};

use super::invocation::{InvocationStatus, ToolInvocationRecord};
use super::registry::ToolRegistry;
use super::schema::{validate_schema, ExecutionContext};

/// Result of a successful tool execution.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// The JSON value returned by the tool handler.
    pub result: serde_json::Value,
    /// The invocation record capturing timing and status.
    pub invocation: ToolInvocationRecord,
}

/// Outcome of the tool execution pipeline, including permission decisions.
#[derive(Debug)]
pub enum ToolExecutionOutcome {
    /// The tool was executed successfully.
    Executed(ExecutionResult),
    /// The tool was denied by the permission system.
    Denied {
        reason: String,
        invocation: ToolInvocationRecord,
    },
    /// The tool requires user confirmation before execution.
    NeedsConfirmation { confirmation_id: String },
}

/// Execute a tool through the full pipeline: lookup, validate, enforce permissions, execute, emit events.
pub fn execute_tool(
    registry: &ToolRegistry,
    workspace: &mut Workspace,
    event_log: &mut dyn EventLog,
    clipboard_store: &mut dyn ClipboardStore,
    permission_store: &PermissionStore,
    instance_id: &str,
    tool_id: &str,
    args: serde_json::Value,
    tool_call_id: Uuid,
) -> ToolExecutionOutcome {
    // 1. Lookup
    let tool = match registry.lookup(instance_id, tool_id) {
        Some(t) => t,
        None => {
            let now = SystemTime::now();
            let invocation = ToolInvocationRecord::new(
                tool_id.to_string(),
                now,
                now,
                InvocationStatus::Failed,
                String::new(),
                String::new(),
            );
            return ToolExecutionOutcome::Denied {
                reason: format!("unknown tool: '{tool_id}'"),
                invocation,
            };
        }
    };

    // 2. Validate input args
    if let Err(e) = validate_schema(&args, &tool.input_schema) {
        let now = SystemTime::now();
        let invocation = ToolInvocationRecord::new(
            tool_id.to_string(),
            now,
            now,
            InvocationStatus::Failed,
            String::new(),
            String::new(),
        );
        return ToolExecutionOutcome::Denied {
            reason: format!("{e}"),
            invocation,
        };
    }

    // 3. Enforce permissions
    match enforce_permissions(tool, permission_store, tool_call_id) {
        EnforcementResult::Allowed => {
            // Emit authorized event
            event_log.append(Event::ToolCallAuthorized {
                id: Uuid::new_v4(),
                timestamp: SystemTime::now(),
                tool_call_id,
            });
        }
        EnforcementResult::Denied { reason } => {
            let now = SystemTime::now();
            let invocation = ToolInvocationRecord::new(
                tool_id.to_string(),
                now,
                now,
                InvocationStatus::Failed,
                workspace_hash(workspace),
                workspace_hash(workspace),
            );
            // Emit denied event
            event_log.append(Event::ToolCallDenied {
                id: Uuid::new_v4(),
                timestamp: SystemTime::now(),
                tool_call_id,
                reason: reason.clone(),
            });
            return ToolExecutionOutcome::Denied { reason, invocation };
        }
        EnforcementResult::NeedsConfirmation { confirmation_id } => {
            // Transition workspace to AwaitingConfirmation
            workspace.mode = WorkspaceMode::AwaitingConfirmation;
            workspace.confirmation_pending = Some(ConfirmationPending {
                confirmation_id: confirmation_id.clone(),
                tool_id: tool_id.to_string(),
                args: args.clone(),
                requested_at: SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
            return ToolExecutionOutcome::NeedsConfirmation { confirmation_id };
        }
    }

    // 4. Capture hash_before
    let hash_before = workspace_hash(workspace);

    // 5. Execute handler
    let started_at = SystemTime::now();
    let mut ctx = ExecutionContext {
        workspace,
        event_log,
        clipboard_store,
    };
    let handler_result = (tool.handler)(&args, &mut ctx);
    let ended_at = SystemTime::now();

    // After handler, re-borrow workspace and event_log from ctx
    let workspace = ctx.workspace;
    let event_log = ctx.event_log;

    // 6. Capture hash_after
    let hash_after = workspace_hash(workspace);

    let is_kernel = tool.is_kernel;

    match handler_result {
        Ok(result) => {
            // 7. Build invocation record (success)
            let invocation = ToolInvocationRecord::new(
                tool_id.to_string(),
                started_at,
                ended_at,
                InvocationStatus::Success,
                hash_before.clone(),
                hash_after.clone(),
            );

            // 8. Emit events
            event_log.append(Event::ToolCallExecuted {
                id: Uuid::new_v4(),
                timestamp: SystemTime::now(),
                tool_call_id,
                invocation: invocation.clone(),
            });

            event_log.append(Event::ToolResultRecorded {
                id: Uuid::new_v4(),
                timestamp: SystemTime::now(),
                tool_call_id,
                result: result.clone(),
            });

            // WorkspacePatched only for kernel tools that changed the workspace
            if is_kernel && hash_before != hash_after {
                event_log.append(Event::WorkspacePatched {
                    id: Uuid::new_v4(),
                    timestamp: SystemTime::now(),
                    patch: WorkspacePatch {
                        operations: vec![], // v0: handler mutates directly
                    },
                    workspace_hash_before: hash_before,
                    workspace_hash_after: hash_after,
                });
            }

            // 9. Return
            ToolExecutionOutcome::Executed(ExecutionResult { result, invocation })
        }
        Err(e) => {
            // 7. Build invocation record (failure)
            let mut invocation = ToolInvocationRecord::new(
                tool_id.to_string(),
                started_at,
                ended_at,
                InvocationStatus::Failed,
                hash_before,
                hash_after,
            );
            invocation.error_code = Some(format!("{e}"));

            // 8. Emit ToolCallExecuted with Failed status
            event_log.append(Event::ToolCallExecuted {
                id: Uuid::new_v4(),
                timestamp: SystemTime::now(),
                tool_call_id,
                invocation: invocation.clone(),
            });

            ToolExecutionOutcome::Denied {
                reason: format!("tool '{tool_id}' failed: {e}"),
                invocation,
            }
        }
    }
}

/// Simple byte-sum hash of the serialized workspace JSON for change detection.
fn workspace_hash(workspace: &Workspace) -> String {
    let json = serde_json::to_string(workspace).unwrap_or_default();
    let sum: u64 = json.bytes().map(|b| b as u64).sum();
    format!("{:016x}", sum)
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CoreError;
    use crate::events::Event;
    use crate::permissions::PermissionStore;
    use crate::storage::{MemoryStorage, Storage};
    use crate::tools::schema::{RiskLevel, ToolDefinition, ToolHandler};
    use crate::tools::registry::ToolRegistry;
    use serde_json::json;

    fn make_handler_ok(result: serde_json::Value) -> ToolHandler {
        Box::new(move |_args, _ctx| Ok(result.clone()))
    }

    fn make_handler_fail() -> ToolHandler {
        Box::new(|_args, _ctx| Err(CoreError::Internal("handler exploded".to_string())))
    }

    fn make_handler_mutates_workspace() -> ToolHandler {
        Box::new(|_args, ctx| {
            ctx.workspace.session_id = "mutated".to_string();
            Ok(json!({"mutated": true}))
        })
    }

    fn make_tool(id: &str, is_kernel: bool, handler: ToolHandler) -> ToolDefinition {
        ToolDefinition {
            tool_id: id.to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["name"],
                "properties": {
                    "name": {"type": "string"}
                }
            }),
            output_schema: json!({}),
            risk_level: RiskLevel::Safe,
            is_kernel,
            handler,
        }
    }

    fn setup() -> (ToolRegistry, Workspace, Box<dyn Storage>, PermissionStore) {
        let registry = ToolRegistry::new();
        let workspace = Workspace::new("test-session".to_string());
        let storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let permission_store = PermissionStore::new();
        (registry, workspace, storage, permission_store)
    }

    #[test]
    fn unknown_tool_returns_denied() {
        let (registry, mut workspace, mut storage, permission_store) = setup();
        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let result = execute_tool(
            &registry, &mut workspace, event_log, clipboard_store, &permission_store,
            "inst-1", "nonexistent", json!({}), Uuid::new_v4(),
        );
        assert!(matches!(result, ToolExecutionOutcome::Denied { .. }));
    }

    #[test]
    fn invalid_args_returns_denied() {
        let (mut registry, mut workspace, mut storage, permission_store) = setup();
        registry.register_kernel_tool(make_tool("my_tool", true, make_handler_ok(json!("ok"))));

        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        // Missing required "name" field
        let result = execute_tool(
            &registry, &mut workspace, event_log, clipboard_store, &permission_store,
            "inst-1", "my_tool", json!({}), Uuid::new_v4(),
        );
        assert!(matches!(result, ToolExecutionOutcome::Denied { .. }));
    }

    #[test]
    fn valid_tool_returns_correct_result() {
        let (mut registry, mut workspace, mut storage, permission_store) = setup();
        registry.register_kernel_tool(make_tool(
            "echo", true, make_handler_ok(json!({"echo": "hello"})),
        ));

        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let result = execute_tool(
            &registry, &mut workspace, event_log, clipboard_store, &permission_store,
            "inst-1", "echo", json!({"name": "test"}), Uuid::new_v4(),
        );

        if let ToolExecutionOutcome::Executed(exec) = result {
            assert_eq!(exec.result, json!({"echo": "hello"}));
        } else {
            panic!("expected Executed outcome");
        }
    }

    #[test]
    fn invocation_record_has_correct_fields() {
        let (mut registry, mut workspace, mut storage, permission_store) = setup();
        registry.register_kernel_tool(make_tool(
            "my_tool", true, make_handler_ok(json!(null)),
        ));

        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let result = execute_tool(
            &registry, &mut workspace, event_log, clipboard_store, &permission_store,
            "inst-1", "my_tool", json!({"name": "x"}), Uuid::new_v4(),
        );

        if let ToolExecutionOutcome::Executed(exec) = result {
            assert_eq!(exec.invocation.tool_id, "my_tool");
            assert_eq!(exec.invocation.status, InvocationStatus::Success);
            assert!(exec.invocation.error_code.is_none());
        } else {
            panic!("expected Executed outcome");
        }
    }

    #[test]
    fn tool_call_executed_event_emitted() {
        let (mut registry, mut workspace, mut storage, permission_store) = setup();
        registry.register_kernel_tool(make_tool(
            "my_tool", true, make_handler_ok(json!(null)),
        ));

        let call_id = Uuid::new_v4();
        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        execute_tool(
            &registry, &mut workspace, event_log, clipboard_store, &permission_store,
            "inst-1", "my_tool", json!({"name": "x"}), call_id,
        );

        let records = storage.event_log().tail(100);
        let executed = records.iter().find(|r| matches!(&r.event, Event::ToolCallExecuted { .. }));
        assert!(executed.is_some());
        if let Event::ToolCallExecuted { tool_call_id, invocation, .. } = &executed.unwrap().event {
            assert_eq!(*tool_call_id, call_id);
            assert_eq!(invocation.status, InvocationStatus::Success);
        }
    }

    #[test]
    fn tool_result_recorded_event_emitted_on_success() {
        let (mut registry, mut workspace, mut storage, permission_store) = setup();
        registry.register_kernel_tool(make_tool(
            "my_tool", true, make_handler_ok(json!({"data": 42})),
        ));

        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        execute_tool(
            &registry, &mut workspace, event_log, clipboard_store, &permission_store,
            "inst-1", "my_tool", json!({"name": "x"}), Uuid::new_v4(),
        );

        let records = storage.event_log().tail(100);
        let recorded = records.iter().find(|r| matches!(&r.event, Event::ToolResultRecorded { .. }));
        assert!(recorded.is_some());
        if let Event::ToolResultRecorded { result, .. } = &recorded.unwrap().event {
            assert_eq!(*result, json!({"data": 42}));
        }
    }

    #[test]
    fn kernel_tool_emits_workspace_patched_on_mutation() {
        let (mut registry, mut workspace, mut storage, permission_store) = setup();
        let tool = ToolDefinition {
            tool_id: "mutator".to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            risk_level: RiskLevel::Safe,
            is_kernel: true,
            handler: make_handler_mutates_workspace(),
        };
        registry.register_kernel_tool(tool);

        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let result = execute_tool(
            &registry, &mut workspace, event_log, clipboard_store, &permission_store,
            "inst-1", "mutator", json!({}), Uuid::new_v4(),
        );
        assert!(matches!(result, ToolExecutionOutcome::Executed(_)));

        let records = storage.event_log().tail(100);
        let patched = records.iter().find(|r| matches!(&r.event, Event::WorkspacePatched { .. }));
        assert!(patched.is_some());
    }

    #[test]
    fn kernel_tool_noop_does_not_emit_workspace_patched() {
        let (mut registry, mut workspace, mut storage, permission_store) = setup();
        let tool = ToolDefinition {
            tool_id: "noop".to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            risk_level: RiskLevel::Safe,
            is_kernel: true,
            handler: make_handler_ok(json!(null)),
        };
        registry.register_kernel_tool(tool);

        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        execute_tool(
            &registry, &mut workspace, event_log, clipboard_store, &permission_store,
            "inst-1", "noop", json!({}), Uuid::new_v4(),
        );

        let records = storage.event_log().tail(100);
        let patched = records.iter().any(|r| matches!(&r.event, Event::WorkspacePatched { .. }));
        assert!(!patched);
    }

    #[test]
    fn handler_failure_returns_denied() {
        let (mut registry, mut workspace, mut storage, permission_store) = setup();
        let tool = ToolDefinition {
            tool_id: "fail_tool".to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            risk_level: RiskLevel::Safe,
            is_kernel: false,
            handler: make_handler_fail(),
        };
        registry.register_instance_tool("inst-1".to_string(), tool);

        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let result = execute_tool(
            &registry, &mut workspace, event_log, clipboard_store, &permission_store,
            "inst-1", "fail_tool", json!({}), Uuid::new_v4(),
        );
        assert!(matches!(result, ToolExecutionOutcome::Denied { .. }));
    }

    #[test]
    fn failed_handler_still_emits_tool_call_executed() {
        let (mut registry, mut workspace, mut storage, permission_store) = setup();
        let tool = ToolDefinition {
            tool_id: "fail_tool".to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            risk_level: RiskLevel::Safe,
            is_kernel: false,
            handler: make_handler_fail(),
        };
        registry.register_instance_tool("inst-1".to_string(), tool);

        let call_id = Uuid::new_v4();
        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        execute_tool(
            &registry, &mut workspace, event_log, clipboard_store, &permission_store,
            "inst-1", "fail_tool", json!({}), call_id,
        );

        let records = storage.event_log().tail(100);
        let executed = records.iter().find(|r| matches!(&r.event, Event::ToolCallExecuted { .. }));
        assert!(executed.is_some());
        if let Event::ToolCallExecuted { tool_call_id, invocation, .. } = &executed.unwrap().event {
            assert_eq!(*tool_call_id, call_id);
            assert_eq!(invocation.status, InvocationStatus::Failed);
            assert!(invocation.error_code.is_some());
        }
    }

    // --- Permission-specific tests ---

    #[test]
    fn safe_tool_empty_store_executes() {
        let (mut registry, mut workspace, mut storage, permission_store) = setup();
        let tool = ToolDefinition {
            tool_id: "safe_tool".to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            risk_level: RiskLevel::Safe,
            is_kernel: false,
            handler: make_handler_ok(json!("ok")),
        };
        registry.register_instance_tool("inst-1".to_string(), tool);

        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let result = execute_tool(
            &registry, &mut workspace, event_log, clipboard_store, &permission_store,
            "inst-1", "safe_tool", json!({}), Uuid::new_v4(),
        );
        assert!(matches!(result, ToolExecutionOutcome::Executed(_)));
    }

    #[test]
    fn destructive_tool_empty_store_needs_confirmation() {
        let (mut registry, mut workspace, mut storage, permission_store) = setup();
        let tool = ToolDefinition {
            tool_id: "delete_all".to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            risk_level: RiskLevel::Destructive,
            is_kernel: false,
            handler: make_handler_ok(json!("deleted")),
        };
        registry.register_instance_tool("inst-1".to_string(), tool);

        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let result = execute_tool(
            &registry, &mut workspace, event_log, clipboard_store, &permission_store,
            "inst-1", "delete_all", json!({}), Uuid::new_v4(),
        );
        assert!(matches!(result, ToolExecutionOutcome::NeedsConfirmation { .. }));
    }

    #[test]
    fn confirm_tool_empty_store_needs_confirmation() {
        let (mut registry, mut workspace, mut storage, permission_store) = setup();
        let tool = ToolDefinition {
            tool_id: "write_file".to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            risk_level: RiskLevel::Confirm,
            is_kernel: false,
            handler: make_handler_ok(json!("written")),
        };
        registry.register_instance_tool("inst-1".to_string(), tool);

        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let result = execute_tool(
            &registry, &mut workspace, event_log, clipboard_store, &permission_store,
            "inst-1", "write_file", json!({}), Uuid::new_v4(),
        );
        assert!(matches!(result, ToolExecutionOutcome::NeedsConfirmation { .. }));
    }

    #[test]
    fn stored_deny_returns_denied_with_event() {
        let (mut registry, mut workspace, mut storage, mut permission_store) = setup();
        let tool = ToolDefinition {
            tool_id: "write_file".to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            risk_level: RiskLevel::Confirm,
            is_kernel: false,
            handler: make_handler_ok(json!("written")),
        };
        registry.register_instance_tool("inst-1".to_string(), tool);

        use crate::permissions::{PermissionDecision, PermissionScope};
        permission_store.set_decision(
            "write_file".to_string(),
            PermissionScope::Write,
            PermissionDecision::Deny,
        );

        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let result = execute_tool(
            &registry, &mut workspace, event_log, clipboard_store, &permission_store,
            "inst-1", "write_file", json!({}), Uuid::new_v4(),
        );
        assert!(matches!(result, ToolExecutionOutcome::Denied { .. }));

        // ToolCallDenied event should be emitted
        let records = storage.event_log().tail(100);
        let denied = records.iter().find(|r| matches!(&r.event, Event::ToolCallDenied { .. }));
        assert!(denied.is_some());

        // Workspace mode should NOT change for denied tools
        assert_eq!(workspace.mode, WorkspaceMode::Idle);
    }

    #[test]
    fn stored_allow_executes_confirm_tool() {
        let (mut registry, mut workspace, mut storage, mut permission_store) = setup();
        let tool = ToolDefinition {
            tool_id: "write_file".to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            risk_level: RiskLevel::Confirm,
            is_kernel: false,
            handler: make_handler_ok(json!("written")),
        };
        registry.register_instance_tool("inst-1".to_string(), tool);

        use crate::permissions::{PermissionDecision, PermissionScope};
        permission_store.set_decision(
            "write_file".to_string(),
            PermissionScope::Write,
            PermissionDecision::Allow,
        );

        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let result = execute_tool(
            &registry, &mut workspace, event_log, clipboard_store, &permission_store,
            "inst-1", "write_file", json!({}), Uuid::new_v4(),
        );
        assert!(matches!(result, ToolExecutionOutcome::Executed(_)));
    }

    #[test]
    fn needs_confirmation_sets_workspace_state() {
        let (mut registry, mut workspace, mut storage, permission_store) = setup();
        let tool = ToolDefinition {
            tool_id: "risky_tool".to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            risk_level: RiskLevel::Destructive,
            is_kernel: false,
            handler: make_handler_ok(json!("done")),
        };
        registry.register_instance_tool("inst-1".to_string(), tool);

        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let result = execute_tool(
            &registry, &mut workspace, event_log, clipboard_store, &permission_store,
            "inst-1", "risky_tool", json!({}), Uuid::new_v4(),
        );

        assert!(matches!(result, ToolExecutionOutcome::NeedsConfirmation { .. }));
        assert_eq!(workspace.mode, WorkspaceMode::AwaitingConfirmation);
        assert!(workspace.confirmation_pending.is_some());
        let pending = workspace.confirmation_pending.as_ref().unwrap();
        assert_eq!(pending.tool_id, "risky_tool");
    }

    #[test]
    fn tool_call_authorized_event_emitted_for_allowed() {
        let (mut registry, mut workspace, mut storage, permission_store) = setup();
        let tool = ToolDefinition {
            tool_id: "safe_tool".to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            risk_level: RiskLevel::Safe,
            is_kernel: false,
            handler: make_handler_ok(json!("ok")),
        };
        registry.register_instance_tool("inst-1".to_string(), tool);

        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        execute_tool(
            &registry, &mut workspace, event_log, clipboard_store, &permission_store,
            "inst-1", "safe_tool", json!({}), Uuid::new_v4(),
        );

        let records = storage.event_log().tail(100);
        let authorized = records.iter().find(|r| matches!(&r.event, Event::ToolCallAuthorized { .. }));
        assert!(authorized.is_some());
    }
}
