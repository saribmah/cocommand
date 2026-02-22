use std::sync::Arc;

use async_trait::async_trait;
use futures_util::stream;
use serde_json::json;
use tempfile::tempdir;
use tokio::sync::{mpsc, Semaphore};
use tokio::time::{timeout, Duration};

use super::*;
use crate::bus::Bus;
use crate::command::runtime::protocol::ToolExecutionContext;
use crate::llm::{LlmError, LlmProvider, LlmSettings, LlmStream, LlmStreamEvent, LlmStreamOptions};
use crate::message::message::MessageStorage;
use crate::workspace::WorkspaceInstance;

#[derive(Clone)]
struct FakeLlmProvider;

#[async_trait]
impl LlmProvider for FakeLlmProvider {
    async fn stream(
        &self,
        _messages: &[crate::message::Message],
        _tools: crate::llm::LlmToolSet,
    ) -> Result<LlmStream, LlmError> {
        self.stream_with_options(
            &[],
            crate::llm::LlmToolSet::new(),
            LlmStreamOptions::default(),
        )
        .await
    }

    async fn stream_with_options(
        &self,
        _messages: &[crate::message::Message],
        _tools: crate::llm::LlmToolSet,
        _options: LlmStreamOptions,
    ) -> Result<LlmStream, LlmError> {
        let events = vec![
            LlmStreamEvent::TextDelta {
                id: "text-1".to_string(),
                text: "hello".to_string(),
            },
            LlmStreamEvent::Finish,
        ];
        Ok(Box::pin(stream::iter(events)))
    }

    async fn update_settings(&self, _settings: LlmSettings) -> Result<(), LlmError> {
        Ok(())
    }

    fn with_settings(&self, _settings: LlmSettings) -> Result<Box<dyn LlmProvider>, LlmError> {
        Ok(Box::new(self.clone()))
    }
}

fn test_semaphores() -> RuntimeSemaphores {
    RuntimeSemaphores {
        llm: Arc::new(Semaphore::new(2)),
        tool: Arc::new(Semaphore::new(2)),
    }
}

#[tokio::test]
async fn llm_command_emits_finished_event_with_mapped_parts() {
    let dir = tempdir().expect("tempdir");
    let workspace = WorkspaceInstance::new(dir.path()).await.expect("workspace");
    let bus = Bus::new(16);

    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    spawn_runtime_executor(
        Arc::new(FakeLlmProvider),
        test_semaphores(),
        workspace.storage.clone(),
        bus,
        "session-1".to_string(),
        command_rx,
        event_tx,
    );

    let assistant = crate::message::Message::from_parts("session-1", "assistant", Vec::new());
    MessageStorage::store_info(&workspace.storage, &assistant.info)
        .await
        .expect("store assistant info");

    command_tx
        .send(RuntimeCommand::CallLlm {
            run_id: "run-1".to_string(),
            assistant_message_id: assistant.info.id.clone(),
            messages: Vec::new(),
            tools: crate::llm::LlmToolSet::new(),
            cancel_token: tokio_util::sync::CancellationToken::new(),
        })
        .expect("send command");

    let event = timeout(Duration::from_secs(2), event_rx.recv())
        .await
        .expect("event")
        .expect("payload");

    assert!(matches!(
        event,
        SessionEvent::LlmFinished { ref run_id, ref parts }
            if run_id == "run-1" && !parts.is_empty()
    ));
}

#[tokio::test]
async fn missing_tool_emits_immediate_failure() {
    let dir = tempdir().expect("tempdir");
    let workspace = WorkspaceInstance::new(dir.path()).await.expect("workspace");
    let bus = Bus::new(16);

    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    spawn_runtime_executor(
        Arc::new(FakeLlmProvider),
        test_semaphores(),
        workspace.storage.clone(),
        bus,
        "session-1".to_string(),
        command_rx,
        event_tx,
    );

    command_tx
        .send(RuntimeCommand::CallTool {
            context: ToolExecutionContext {
                session_id: "session-1".to_string(),
                run_id: "run-1".to_string(),
                message_id: "message-1".to_string(),
                part_id: "part-1".to_string(),
                tool_call_id: "tool-call-1".to_string(),
                tool_name: "missing_tool".to_string(),
                input: serde_json::Map::new(),
                started_at: 1,
            },
            input: json!({"x": 1}),
            tool: None,
        })
        .expect("send command");

    let event = timeout(Duration::from_secs(2), event_rx.recv())
        .await
        .expect("event")
        .expect("payload");

    assert!(matches!(
        event,
        SessionEvent::ToolFailure(ref payload)
            if payload.run_id == "run-1" && payload.tool_call_id == "tool-call-1"
    ));
}
