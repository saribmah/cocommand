use std::sync::Arc;

use tokio::sync::mpsc;
use tokio_stream::StreamExt;

use crate::command::runtime::protocol::{RuntimeCommand, SessionEvent};
use crate::command::runtime::tool_worker::spawn_tool_execution;
use crate::command::runtime::types::RuntimeSemaphores;
use crate::llm::{LlmProvider, LlmStreamOptions};

pub fn spawn_runtime_executor(
    llm: Arc<dyn LlmProvider>,
    semaphores: RuntimeSemaphores,
    mut command_rx: mpsc::UnboundedReceiver<RuntimeCommand>,
    event_tx: mpsc::UnboundedSender<SessionEvent>,
) {
    tokio::spawn(async move {
        while let Some(command) = command_rx.recv().await {
            match command {
                RuntimeCommand::CallLlm {
                    run_id,
                    messages,
                    tools,
                    cancel_token,
                } => {
                    spawn_llm_execution(
                        llm.clone(),
                        semaphores.clone(),
                        event_tx.clone(),
                        run_id,
                        messages,
                        tools,
                        cancel_token,
                    );
                }
                RuntimeCommand::CallTool {
                    run_id,
                    tool_call_id,
                    tool_name,
                    input,
                    tool,
                    is_async,
                } => {
                    spawn_tool_execution(
                        event_tx.clone(),
                        semaphores.clone(),
                        run_id,
                        tool_call_id,
                        tool_name,
                        input,
                        tool,
                        is_async,
                    );
                }
            }
        }
    });
}

fn spawn_llm_execution(
    llm: Arc<dyn LlmProvider>,
    semaphores: RuntimeSemaphores,
    event_tx: mpsc::UnboundedSender<SessionEvent>,
    run_id: String,
    messages: Vec<crate::message::Message>,
    tools: crate::llm::LlmToolSet,
    cancel_token: tokio_util::sync::CancellationToken,
) {
    tokio::spawn(async move {
        let _permit = match semaphores.llm.acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => return,
        };

        let stream = llm
            .stream_with_options(
                &messages,
                tools,
                LlmStreamOptions {
                    max_steps: Some(1),
                    abort_signal: Some(cancel_token.clone()),
                },
            )
            .await;

        let mut stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                let _ = event_tx.send(SessionEvent::LlmFailed {
                    run_id,
                    error: error.to_string(),
                    cancelled: cancel_token.is_cancelled(),
                });
                return;
            }
        };

        while let Some(part) = stream.next().await {
            if event_tx
                .send(SessionEvent::LlmStreamPart {
                    run_id: run_id.clone(),
                    part,
                })
                .is_err()
            {
                return;
            }
        }

        let _ = event_tx.send(SessionEvent::LlmFinished { run_id });
    });
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use futures_util::stream;
    use serde_json::json;
    use tokio::sync::{mpsc, Semaphore};
    use tokio::time::{timeout, Duration};

    use super::*;
    use crate::llm::{LlmError, LlmProvider, LlmSettings, LlmStream, LlmStreamEvent};

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
            jobs: Arc::new(Semaphore::new(2)),
        }
    }

    #[tokio::test]
    async fn llm_command_emits_stream_and_finished_events() {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        spawn_runtime_executor(
            Arc::new(FakeLlmProvider),
            test_semaphores(),
            command_rx,
            event_tx,
        );

        command_tx
            .send(RuntimeCommand::CallLlm {
                run_id: "run-1".to_string(),
                messages: Vec::new(),
                tools: crate::llm::LlmToolSet::new(),
                cancel_token: tokio_util::sync::CancellationToken::new(),
            })
            .expect("send command");

        let first = timeout(Duration::from_secs(2), event_rx.recv())
            .await
            .expect("first event")
            .expect("first payload");
        assert!(matches!(
            first,
            SessionEvent::LlmStreamPart { ref run_id, .. } if run_id == "run-1"
        ));

        let second = timeout(Duration::from_secs(2), event_rx.recv())
            .await
            .expect("second event")
            .expect("second payload");
        assert!(matches!(
            second,
            SessionEvent::LlmStreamPart { ref run_id, part: LlmStreamEvent::Finish } if run_id == "run-1"
        ));

        let third = timeout(Duration::from_secs(2), event_rx.recv())
            .await
            .expect("third event")
            .expect("third payload");
        assert!(matches!(
            third,
            SessionEvent::LlmFinished { ref run_id } if run_id == "run-1"
        ));
    }

    #[tokio::test]
    async fn missing_tool_emits_immediate_failure() {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        spawn_runtime_executor(
            Arc::new(FakeLlmProvider),
            test_semaphores(),
            command_rx,
            event_tx,
        );

        command_tx
            .send(RuntimeCommand::CallTool {
                run_id: "run-1".to_string(),
                tool_call_id: "tool-call-1".to_string(),
                tool_name: "missing_tool".to_string(),
                input: json!({"x": 1}),
                tool: None,
                is_async: false,
            })
            .expect("send command");

        let event = timeout(Duration::from_secs(2), event_rx.recv())
            .await
            .expect("event")
            .expect("payload");

        assert!(matches!(
            event,
            SessionEvent::ToolImmediateFailure(ref payload)
                if payload.run_id == "run-1" && payload.tool_call_id == "tool-call-1"
        ));
    }
}
