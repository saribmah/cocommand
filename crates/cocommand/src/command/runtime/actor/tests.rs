use std::sync::Arc;

use serde_json::json;
use tempfile::tempdir;
use tokio::sync::mpsc;

use super::*;
use crate::llm::{LlmTool, LlmToolSet};
use crate::session::SessionManager;

async fn test_actor() -> (
    SessionRuntimeActor,
    mpsc::UnboundedReceiver<RuntimeCommand>,
    tempfile::TempDir,
) {
    let dir = tempdir().expect("tempdir");
    let workspace = WorkspaceInstance::new(dir.path()).await.expect("workspace");
    let workspace_arc = Arc::new(workspace.clone());
    let sessions = Arc::new(SessionManager::new(workspace_arc));
    let bus = Bus::new(32);

    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (command_tx, command_rx) = mpsc::unbounded_channel();

    (
        SessionRuntimeActor::new(
            "session-1".to_string(),
            workspace,
            sessions,
            bus,
            event_tx,
            event_rx,
            command_tx,
        ),
        command_rx,
        dir,
    )
}

#[tokio::test]
async fn dispatch_tool_call_emits_runtime_command() {
    let (actor, mut command_rx, _dir) = test_actor().await;
    let mut tools = LlmToolSet::new();
    tools.insert(
        "example_tool".to_string(),
        LlmTool {
            description: Some("tool".to_string()),
            input_schema: json!({"type": "object"}),
            execute: None,
        },
    );

    actor
        .dispatch_tool_call(
            &tools,
            ToolExecutionContext {
                session_id: "session-1".to_string(),
                run_id: "run-1".to_string(),
                message_id: "message-1".to_string(),
                part_id: "part-1".to_string(),
                tool_call_id: "tool-call-1".to_string(),
                tool_name: "example_tool".to_string(),
                input: serde_json::Map::new(),
                started_at: 1,
            },
            json!({"value": 1}),
        )
        .await;

    let command = command_rx.recv().await.expect("runtime command");
    assert!(matches!(
        command,
        RuntimeCommand::CallTool {
            context,
            ..
        } if context.run_id == "run-1" && context.tool_call_id == "tool-call-1" && context.tool_name == "example_tool"
    ));
}

#[tokio::test]
async fn cancelled_run_ignores_immediate_tool_completion() {
    let (mut actor, _command_rx, _dir) = test_actor().await;
    actor.remember_cancelled_run("run-cancelled".to_string());

    actor
        .handle_tool_immediate_success(ToolImmediateSuccess {
            run_id: "run-cancelled".to_string(),
            tool_call_id: "tool-call-1".to_string(),
        })
        .await
        .expect("ignored completion");

    assert_eq!(actor.queued_followups, 0);
}
