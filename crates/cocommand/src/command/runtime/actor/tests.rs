use std::sync::Arc;

use serde_json::{json, Map};
use tempfile::tempdir;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::*;
use crate::command::processor::StreamProcessor;
use crate::llm::{LlmTool, LlmToolSet};
use crate::message::Message;
use crate::session::SessionManager;
use crate::utils::time::now_secs;

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
    let (mut actor, mut command_rx, _dir) = test_actor().await;
    let mut tools = LlmToolSet::new();
    tools.insert(
        "example_tool".to_string(),
        LlmTool {
            description: Some("tool".to_string()),
            input_schema: json!({"type": "object"}),
            execute: None,
        },
    );

    actor.inflight = Some(RunState {
        run_id: "run-1".to_string(),
        assistant_message: Message::from_parts("session-1", "assistant", Vec::new()),
        processor: StreamProcessor::new(),
        tools,
        cancel_token: CancellationToken::new(),
    });

    actor
        .dispatch_tool_call(
            "run-1".to_string(),
            "tool-call-1".to_string(),
            "example_tool".to_string(),
            json!({"value": 1}),
        )
        .await;

    let command = command_rx.recv().await.expect("runtime command");
    assert!(matches!(
        command,
        RuntimeCommand::CallTool {
            run_id,
            tool_call_id,
            tool_name,
            ..
        } if run_id == "run-1" && tool_call_id == "tool-call-1" && tool_name == "example_tool"
    ));
}

#[tokio::test]
async fn cancelled_run_ignores_immediate_tool_completion() {
    let (mut actor, _command_rx, _dir) = test_actor().await;
    actor.remember_cancelled_run("run-cancelled".to_string());

    actor.tool_calls.insert(
        "tool-call-1".to_string(),
        ToolCallRecord {
            session_id: "session-1".to_string(),
            run_id: "run-cancelled".to_string(),
            message_id: "message-1".to_string(),
            part_id: "part-1".to_string(),
            tool_call_id: "tool-call-1".to_string(),
            tool_name: "example_tool".to_string(),
            input: Map::new(),
            started_at: now_secs(),
        },
    );

    actor
        .handle_tool_immediate_success(ToolImmediateSuccess {
            run_id: "run-cancelled".to_string(),
            tool_call_id: "tool-call-1".to_string(),
            output: json!({"ok": true}),
        })
        .await
        .expect("ignored completion");

    assert_eq!(actor.queued_followups, 0);
}
