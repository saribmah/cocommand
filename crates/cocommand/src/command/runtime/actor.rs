mod helpers;
#[cfg(test)]
mod tests;

use std::collections::VecDeque;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::bus::Bus;
use crate::command::runtime::protocol::{
    RuntimeCommand, SessionEvent, ToolBatchCall, ToolBatchResult, ToolExecutionContext,
};
use crate::command::runtime::types::EnqueueMessageAck;
use crate::command::session_message::{map_input_parts, SessionCommandInputPart};
use crate::error::{CoreError, CoreResult};
use crate::event::{
    CoreEvent, SessionContextPayload, SessionMessageStartedPayload, SessionRunCancelledPayload,
    SessionRunCompletedPayload,
};
use crate::llm::LlmToolSet;
use crate::message::message::MessageStorage;
use crate::message::{Message, MessagePart, ToolPart, ToolState};
use crate::session::SessionManager;
use crate::tool::ToolRegistry;
use crate::utils::time::now_secs;
use crate::workspace::WorkspaceInstance;

use self::helpers::{input_from_tool_state, running_input_and_start, strip_tool_execute};

pub(crate) struct SessionRuntimeActor {
    session_id: String,
    workspace: WorkspaceInstance,
    sessions: Arc<SessionManager>,
    bus: Bus,
    event_tx: mpsc::UnboundedSender<SessionEvent>,
    event_rx: mpsc::UnboundedReceiver<SessionEvent>,
    command_tx: mpsc::UnboundedSender<RuntimeCommand>,
    inflight: Option<RunState>,
    queued_followups: usize,
    pending_user_runs: VecDeque<QueuedUserRun>,
}

struct RunState {
    run_id: String,
    assistant_message: Message,
    tools: LlmToolSet,
    phase: RunPhase,
}

struct QueuedUserRun {
    run_id: String,
    user_message: Message,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RunPhase {
    LlmInFlight,
    AwaitingToolBatch,
}

impl SessionRuntimeActor {
    pub(crate) fn new(
        session_id: String,
        workspace: WorkspaceInstance,
        sessions: Arc<SessionManager>,
        bus: Bus,
        event_tx: mpsc::UnboundedSender<SessionEvent>,
        event_rx: mpsc::UnboundedReceiver<SessionEvent>,
        command_tx: mpsc::UnboundedSender<RuntimeCommand>,
    ) -> Self {
        Self {
            session_id,
            workspace,
            sessions,
            bus,
            event_tx,
            event_rx,
            command_tx,
            inflight: None,
            queued_followups: 0,
            pending_user_runs: VecDeque::new(),
        }
    }

    pub(crate) async fn run(mut self) {
        while let Some(event) = self.event_rx.recv().await {
            let result = match event {
                SessionEvent::UserMessage { parts, reply } => {
                    let result = self.handle_user_message(parts).await;
                    let _ = reply.send(result);
                    Ok(())
                }
                SessionEvent::LlmFinished { run_id, parts } => {
                    self.handle_llm_finished(&run_id, parts).await
                }
                SessionEvent::LlmFailed {
                    run_id,
                    error,
                    cancelled,
                } => self.handle_llm_failed(&run_id, &error, cancelled).await,
                SessionEvent::ToolBatchFinished { run_id, results } => {
                    self.handle_tool_batch_finished(&run_id, results).await
                }
            };

            if let Err(error) = result {
                tracing::warn!(
                    "session runtime actor {} event handling failed: {}",
                    self.session_id,
                    error
                );
            }
        }
    }

    async fn handle_user_message(
        &mut self,
        parts: Vec<SessionCommandInputPart>,
    ) -> CoreResult<EnqueueMessageAck> {
        let run_id = Uuid::now_v7().to_string();
        let mut user_message = Message::from_parts(&self.session_id, "user", Vec::new());
        user_message.parts = map_input_parts(parts, &self.session_id, &user_message.info.id);
        MessageStorage::store(&self.workspace.storage, &user_message).await?;

        self.pending_user_runs.push_back(QueuedUserRun {
            run_id: run_id.clone(),
            user_message,
        });
        self.try_start_next_user_run().await?;

        Ok(EnqueueMessageAck {
            run_id,
            accepted_at: now_secs(),
        })
    }

    async fn start_run(&mut self, user_message: Option<Message>) -> CoreResult<String> {
        let run_id = Uuid::now_v7().to_string();
        self.start_run_with_id(run_id.clone(), user_message).await?;
        Ok(run_id)
    }

    async fn start_run_with_id(
        &mut self,
        run_id: String,
        user_message: Option<Message>,
    ) -> CoreResult<()> {
        if self.inflight.is_some() {
            return Err(CoreError::Internal(
                "attempted to start run while llm already inflight".to_string(),
            ));
        }

        let messages = MessageStorage::load(&self.workspace.storage, &self.session_id).await?;

        let assistant_message = Message::from_parts(&self.session_id, "assistant", Vec::new());
        MessageStorage::store_info(&self.workspace.storage, &assistant_message.info).await?;

        let _ = self.bus.publish(CoreEvent::SessionMessageStarted(
            SessionMessageStartedPayload {
                session_id: self.session_id.clone(),
                run_id: run_id.clone(),
                user_message,
                assistant_message: assistant_message.clone(),
            },
        ));

        let context = self.session_context().await?;
        let _ = self
            .bus
            .publish(CoreEvent::SessionContextUpdated(SessionContextPayload {
                session_id: self.session_id.clone(),
                run_id: Some(run_id.clone()),
                context,
            }));

        let tools = self.build_tools().await?;
        let llm_tools = strip_tool_execute(&tools);
        let cancel_token = CancellationToken::new();

        self.command_tx
            .send(RuntimeCommand::CallLlm {
                run_id: run_id.clone(),
                assistant_message_id: assistant_message.info.id.clone(),
                messages,
                tools: llm_tools,
                cancel_token: cancel_token.clone(),
            })
            .map_err(|_| {
                let _ = self.event_tx.send(SessionEvent::LlmFailed {
                    run_id: run_id.clone(),
                    error: "runtime executor stopped".to_string(),
                    cancelled: false,
                });
                CoreError::Internal("runtime executor stopped".to_string())
            })?;

        self.inflight = Some(RunState {
            run_id: run_id.clone(),
            assistant_message,
            tools,
            phase: RunPhase::LlmInFlight,
        });

        Ok(())
    }

    async fn handle_llm_finished(
        &mut self,
        run_id: &str,
        parts: Vec<MessagePart>,
    ) -> CoreResult<()> {
        let Some(run) = self.inflight.take() else {
            return Ok(());
        };
        if run.run_id != run_id {
            self.inflight = Some(run);
            return Ok(());
        }

        if run.phase != RunPhase::LlmInFlight {
            self.inflight = Some(run);
            return Ok(());
        }

        let calls = self.build_tool_batch_calls(&run, &parts);
        if calls.is_empty() {
            self.complete_run(run).await?;
            return Ok(());
        }
        self.queued_followups = self.queued_followups.saturating_add(1);

        if let Err(error) = self.dispatch_tool_batch(run.run_id.clone(), calls).await {
            self.queued_followups = self.queued_followups.saturating_sub(1);
            let _ = self
                .bus
                .publish(CoreEvent::SessionRunCancelled(SessionRunCancelledPayload {
                    session_id: self.session_id.clone(),
                    run_id: run_id.to_string(),
                    reason: format!("tool_batch_dispatch_failed: {error}"),
                }));
            return Ok(());
        }

        self.inflight = Some(RunState {
            phase: RunPhase::AwaitingToolBatch,
            ..run
        });
        Ok(())
    }

    fn build_tool_batch_calls(&self, run: &RunState, parts: &[MessagePart]) -> Vec<ToolBatchCall> {
        let mut calls = Vec::new();
        for part in parts {
            let MessagePart::Tool(tool_part) = part else {
                continue;
            };
            if !matches!(
                &tool_part.state,
                ToolState::Running(_) | ToolState::Pending(_)
            ) {
                continue;
            }
            let context = self.tool_execution_context(run, tool_part);
            calls.push(ToolBatchCall {
                tool: run.tools.get(&context.tool_name).cloned(),
                context,
                input: Value::Object(input_from_tool_state(&tool_part.state)),
            });
        }
        calls
    }

    async fn dispatch_tool_batch(
        &self,
        run_id: String,
        calls: Vec<ToolBatchCall>,
    ) -> CoreResult<()> {
        self.command_tx
            .send(RuntimeCommand::CallToolBatch { run_id, calls })
            .map_err(|_| {
                CoreError::Internal("runtime executor stopped before calling tools".to_string())
            })
    }

    async fn handle_tool_batch_finished(
        &mut self,
        run_id: &str,
        _results: Vec<ToolBatchResult>,
    ) -> CoreResult<()> {
        let Some(run) = self.inflight.take() else {
            return Ok(());
        };
        if run.run_id != run_id {
            self.inflight = Some(run);
            return Ok(());
        }
        if run.phase != RunPhase::AwaitingToolBatch {
            self.inflight = Some(run);
            return Ok(());
        }

        self.complete_run(run).await
    }

    async fn complete_run(&mut self, mut run: RunState) -> CoreResult<()> {
        MessageStorage::touch_info(&self.workspace.storage, &mut run.assistant_message.info)
            .await?;

        let _ = self
            .bus
            .publish(CoreEvent::SessionRunCompleted(SessionRunCompletedPayload {
                session_id: self.session_id.clone(),
                run_id: run.run_id,
            }));

        if !self.pending_user_runs.is_empty() {
            self.queued_followups = 0;
            self.try_start_next_user_run().await?;
            return Ok(());
        }

        if self.queued_followups > 0 {
            self.queued_followups -= 1;
            let _ = self.start_run(None).await?;
        }

        Ok(())
    }

    fn tool_execution_context(&self, run: &RunState, tool_part: &ToolPart) -> ToolExecutionContext {
        let (input_map, started_at) = running_input_and_start(&tool_part.state, now_secs());
        ToolExecutionContext {
            session_id: self.session_id.clone(),
            run_id: run.run_id.clone(),
            message_id: tool_part.base.message_id.clone(),
            part_id: tool_part.base.id.clone(),
            tool_call_id: tool_part.call_id.clone(),
            tool_name: tool_part.tool.clone(),
            input: input_map,
            started_at,
        }
    }

    async fn handle_llm_failed(
        &mut self,
        run_id: &str,
        error: &str,
        cancelled: bool,
    ) -> CoreResult<()> {
        let Some(run) = self.inflight.as_ref() else {
            return Ok(());
        };
        if run.run_id != run_id {
            return Ok(());
        }
        if run.phase != RunPhase::LlmInFlight {
            return Ok(());
        }

        self.inflight = None;

        let reason = if cancelled {
            "cancelled".to_string()
        } else {
            format!("llm_failed: {error}")
        };
        let _ = self
            .bus
            .publish(CoreEvent::SessionRunCancelled(SessionRunCancelledPayload {
                session_id: self.session_id.clone(),
                run_id: run_id.to_string(),
                reason,
            }));

        self.try_start_next_user_run().await?;

        Ok(())
    }

    async fn try_start_next_user_run(&mut self) -> CoreResult<()> {
        if self.inflight.is_some() {
            return Ok(());
        }
        let Some(next_user) = self.pending_user_runs.pop_front() else {
            return Ok(());
        };
        self.start_run_with_id(next_user.run_id, Some(next_user.user_message))
            .await?;
        Ok(())
    }

    async fn build_tools(&self) -> CoreResult<LlmToolSet> {
        let session_id = self.session_id.clone();
        let active_extension_ids = self
            .sessions
            .with_session_mut(|session| {
                let session_id = session_id.clone();
                Box::pin(async move {
                    if session.session_id != session_id {
                        return Err(CoreError::InvalidInput("session not found".to_string()));
                    }
                    Ok(session.active_extension_ids())
                })
            })
            .await?;

        let tools = ToolRegistry::tools(
            Arc::new(self.workspace.clone()),
            self.sessions.clone(),
            &self.session_id,
            &active_extension_ids,
        )
        .await;

        Ok(tools)
    }

    async fn session_context(&self) -> CoreResult<crate::session::SessionContext> {
        let session_id = self.session_id.clone();
        self.sessions
            .with_session_mut(|session| {
                let session_id = session_id.clone();
                Box::pin(async move {
                    if session.session_id != session_id {
                        return Err(CoreError::InvalidInput("session not found".to_string()));
                    }
                    session.context(None).await
                })
            })
            .await
    }
}
