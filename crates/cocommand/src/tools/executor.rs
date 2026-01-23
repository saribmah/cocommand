//! Tool execution pipeline.

use std::time::SystemTime;
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};
use crate::events::event::Event;
use crate::events::EventStore;
use crate::workspace::{Workspace, WorkspacePatch};

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

/// Execute a tool through the full pipeline: lookup, validate, execute, emit events.
pub fn execute_tool(
    registry: &ToolRegistry,
    workspace: &mut Workspace,
    event_store: &mut EventStore,
    instance_id: &str,
    tool_id: &str,
    args: serde_json::Value,
    tool_call_id: Uuid,
) -> CoreResult<ExecutionResult> {
    // 1. Lookup
    let tool = registry.lookup(instance_id, tool_id).ok_or_else(|| {
        CoreError::InvalidInput(format!("unknown tool: '{tool_id}'"))
    })?;

    // 2. Validate input args
    validate_schema(&args, &tool.input_schema)?;

    // 3. Permissions stub (always allows in v0)
    check_permissions(tool_id, &args)?;

    // 4. Capture hash_before
    let hash_before = workspace_hash(workspace);

    // 5. Execute handler
    let started_at = SystemTime::now();
    let mut ctx = ExecutionContext {
        workspace,
        event_store,
    };
    let handler_result = (tool.handler)(&args, &mut ctx);
    let ended_at = SystemTime::now();

    // After handler, re-borrow workspace and event_store from ctx
    let workspace = ctx.workspace;
    let event_store = ctx.event_store;

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
            event_store.append(Event::ToolCallExecuted {
                id: Uuid::new_v4(),
                timestamp: SystemTime::now(),
                tool_call_id,
                invocation: invocation.clone(),
            });

            event_store.append(Event::ToolResultRecorded {
                id: Uuid::new_v4(),
                timestamp: SystemTime::now(),
                tool_call_id,
                result: result.clone(),
            });

            // WorkspacePatched only for kernel tools that changed the workspace
            if is_kernel && hash_before != hash_after {
                event_store.append(Event::WorkspacePatched {
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
            Ok(ExecutionResult { result, invocation })
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
            event_store.append(Event::ToolCallExecuted {
                id: Uuid::new_v4(),
                timestamp: SystemTime::now(),
                tool_call_id,
                invocation: invocation.clone(),
            });

            Err(CoreError::Internal(format!("tool '{tool_id}' failed: {e}")))
        }
    }
}

/// Simple byte-sum hash of the serialized workspace JSON for change detection.
fn workspace_hash(workspace: &Workspace) -> String {
    let json = serde_json::to_string(workspace).unwrap_or_default();
    let sum: u64 = json.bytes().map(|b| b as u64).sum();
    format!("{:016x}", sum)
}

/// Permissions stub — always allows in v0.
fn check_permissions(_tool_id: &str, _args: &serde_json::Value) -> CoreResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn setup() -> (ToolRegistry, Workspace, EventStore) {
        let registry = ToolRegistry::new();
        let workspace = Workspace::new("test-session".to_string());
        let event_store = EventStore::new();
        (registry, workspace, event_store)
    }

    #[test]
    fn unknown_tool_returns_invalid_input() {
        let (registry, mut workspace, mut event_store) = setup();
        let result = execute_tool(
            &registry, &mut workspace, &mut event_store,
            "inst-1", "nonexistent", json!({}), Uuid::new_v4(),
        );
        assert!(matches!(result, Err(CoreError::InvalidInput(_))));
    }

    #[test]
    fn invalid_args_returns_validation_error() {
        let (mut registry, mut workspace, mut event_store) = setup();
        registry.register_kernel_tool(make_tool("my_tool", true, make_handler_ok(json!("ok"))));

        // Missing required "name" field
        let result = execute_tool(
            &registry, &mut workspace, &mut event_store,
            "inst-1", "my_tool", json!({}), Uuid::new_v4(),
        );
        assert!(matches!(result, Err(CoreError::InvalidInput(_))));
        // No events emitted since we failed before execution
        assert!(event_store.is_empty());
    }

    #[test]
    fn valid_tool_returns_correct_result() {
        let (mut registry, mut workspace, mut event_store) = setup();
        registry.register_kernel_tool(make_tool(
            "echo", true, make_handler_ok(json!({"echo": "hello"})),
        ));

        let result = execute_tool(
            &registry, &mut workspace, &mut event_store,
            "inst-1", "echo", json!({"name": "test"}), Uuid::new_v4(),
        ).unwrap();

        assert_eq!(result.result, json!({"echo": "hello"}));
    }

    #[test]
    fn invocation_record_has_correct_fields() {
        let (mut registry, mut workspace, mut event_store) = setup();
        registry.register_kernel_tool(make_tool(
            "my_tool", true, make_handler_ok(json!(null)),
        ));

        let result = execute_tool(
            &registry, &mut workspace, &mut event_store,
            "inst-1", "my_tool", json!({"name": "x"}), Uuid::new_v4(),
        ).unwrap();

        assert_eq!(result.invocation.tool_id, "my_tool");
        assert_eq!(result.invocation.status, InvocationStatus::Success);
        assert!(result.invocation.error_code.is_none());
    }

    #[test]
    fn tool_call_executed_event_emitted() {
        let (mut registry, mut workspace, mut event_store) = setup();
        registry.register_kernel_tool(make_tool(
            "my_tool", true, make_handler_ok(json!(null)),
        ));

        let call_id = Uuid::new_v4();
        execute_tool(
            &registry, &mut workspace, &mut event_store,
            "inst-1", "my_tool", json!({"name": "x"}), call_id,
        ).unwrap();

        let events = event_store.events();
        let executed = events.iter().find(|e| matches!(e, Event::ToolCallExecuted { .. }));
        assert!(executed.is_some());
        if let Event::ToolCallExecuted { tool_call_id, invocation, .. } = executed.unwrap() {
            assert_eq!(*tool_call_id, call_id);
            assert_eq!(invocation.status, InvocationStatus::Success);
        }
    }

    #[test]
    fn tool_result_recorded_event_emitted_on_success() {
        let (mut registry, mut workspace, mut event_store) = setup();
        registry.register_kernel_tool(make_tool(
            "my_tool", true, make_handler_ok(json!({"data": 42})),
        ));

        execute_tool(
            &registry, &mut workspace, &mut event_store,
            "inst-1", "my_tool", json!({"name": "x"}), Uuid::new_v4(),
        ).unwrap();

        let events = event_store.events();
        let recorded = events.iter().find(|e| matches!(e, Event::ToolResultRecorded { .. }));
        assert!(recorded.is_some());
        if let Event::ToolResultRecorded { result, .. } = recorded.unwrap() {
            assert_eq!(*result, json!({"data": 42}));
        }
    }

    #[test]
    fn kernel_tool_emits_workspace_patched_on_mutation() {
        let (mut registry, mut workspace, mut event_store) = setup();
        let tool = ToolDefinition {
            tool_id: "mutator".to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            risk_level: RiskLevel::Safe,
            is_kernel: true,
            handler: make_handler_mutates_workspace(),
        };
        registry.register_kernel_tool(tool);

        execute_tool(
            &registry, &mut workspace, &mut event_store,
            "inst-1", "mutator", json!({}), Uuid::new_v4(),
        ).unwrap();

        let events = event_store.events();
        let patched = events.iter().find(|e| matches!(e, Event::WorkspacePatched { .. }));
        assert!(patched.is_some());
    }

    #[test]
    fn kernel_tool_noop_does_not_emit_workspace_patched() {
        let (mut registry, mut workspace, mut event_store) = setup();
        let tool = ToolDefinition {
            tool_id: "noop".to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            risk_level: RiskLevel::Safe,
            is_kernel: true,
            handler: make_handler_ok(json!(null)),
        };
        registry.register_kernel_tool(tool);

        execute_tool(
            &registry, &mut workspace, &mut event_store,
            "inst-1", "noop", json!({}), Uuid::new_v4(),
        ).unwrap();

        let events = event_store.events();
        let patched = events.iter().any(|e| matches!(e, Event::WorkspacePatched { .. }));
        assert!(!patched);
    }

    #[test]
    fn handler_failure_returns_internal_error() {
        let (mut registry, mut workspace, mut event_store) = setup();
        let tool = ToolDefinition {
            tool_id: "fail_tool".to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            risk_level: RiskLevel::Safe,
            is_kernel: false,
            handler: make_handler_fail(),
        };
        registry.register_instance_tool("inst-1".to_string(), tool);

        let result = execute_tool(
            &registry, &mut workspace, &mut event_store,
            "inst-1", "fail_tool", json!({}), Uuid::new_v4(),
        );
        assert!(matches!(result, Err(CoreError::Internal(_))));
    }

    #[test]
    fn failed_handler_still_emits_tool_call_executed() {
        let (mut registry, mut workspace, mut event_store) = setup();
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
        let _ = execute_tool(
            &registry, &mut workspace, &mut event_store,
            "inst-1", "fail_tool", json!({}), call_id,
        );

        let events = event_store.events();
        let executed = events.iter().find(|e| matches!(e, Event::ToolCallExecuted { .. }));
        assert!(executed.is_some());
        if let Event::ToolCallExecuted { tool_call_id, invocation, .. } = executed.unwrap() {
            assert_eq!(*tool_call_id, call_id);
            assert_eq!(invocation.status, InvocationStatus::Failed);
            assert!(invocation.error_code.is_some());
        }
    }
}
