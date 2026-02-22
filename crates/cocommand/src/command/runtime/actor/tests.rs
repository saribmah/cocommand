use std::sync::Arc;

use serde_json::json;
use tempfile::tempdir;
use tokio::sync::mpsc;

use super::*;
use crate::command::session_message::{SessionCommandInputPart, SessionCommandTextPartInput};
use crate::llm::LlmTool;
use crate::message::Message;
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
async fn dispatch_tool_batch_emits_runtime_command() {
    let (actor, mut command_rx, _dir) = test_actor().await;
    let call = ToolBatchCall {
        context: ToolExecutionContext {
            session_id: "session-1".to_string(),
            run_id: "run-1".to_string(),
            message_id: "message-1".to_string(),
            part_id: "part-1".to_string(),
            tool_call_id: "tool-call-1".to_string(),
            tool_name: "example_tool".to_string(),
            input: serde_json::Map::new(),
            started_at: 1,
        },
        input: json!({"value": 1}),
        tool: Some(LlmTool {
            description: Some("tool".to_string()),
            input_schema: json!({"type": "object"}),
            execute: None,
        }),
    };

    actor
        .dispatch_tool_batch("run-1".to_string(), vec![call])
        .await
        .expect("dispatch batch");

    let command = command_rx.recv().await.expect("runtime command");
    assert!(matches!(
        command,
        RuntimeCommand::CallToolBatch { run_id, calls }
            if run_id == "run-1" && calls.len() == 1 && calls[0].context.tool_call_id == "tool-call-1"
    ));
}

#[tokio::test]
async fn stale_tool_batch_finished_event_is_ignored() {
    let (mut actor, _command_rx, _dir) = test_actor().await;
    actor.inflight = Some(RunState {
        run_id: "run-active".to_string(),
        assistant_message: Message::from_parts("session-1", "assistant", Vec::new()),
        tools: crate::llm::LlmToolSet::new(),
        phase: RunPhase::AwaitingToolBatch,
    });

    actor
        .handle_tool_batch_finished("run-stale", Vec::new())
        .await
        .expect("ignored stale batch");

    assert!(actor.inflight.is_some());
    assert_eq!(actor.queued_followups, 0);
}

#[tokio::test]
async fn user_message_is_queued_when_run_inflight() {
    let (mut actor, _command_rx, _dir) = test_actor().await;
    actor.inflight = Some(RunState {
        run_id: "run-active".to_string(),
        assistant_message: Message::from_parts("session-1", "assistant", Vec::new()),
        tools: crate::llm::LlmToolSet::new(),
        phase: RunPhase::AwaitingToolBatch,
    });

    let ack = actor
        .handle_user_message(vec![SessionCommandInputPart::Text(
            SessionCommandTextPartInput {
                text: "new user input".to_string(),
            },
        )])
        .await
        .expect("enqueue user message");

    assert!(actor.inflight.is_some());
    assert_eq!(actor.pending_user_runs.len(), 1);
    assert_eq!(actor.pending_user_runs[0].run_id, ack.run_id);
}
