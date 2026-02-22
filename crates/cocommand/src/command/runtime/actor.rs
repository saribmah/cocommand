use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use serde_json::{Map, Value};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::bus::Bus;
use crate::command::processor::{StorePartContext, StreamProcessor};
use crate::command::runtime::executor::spawn_runtime_executor;
use crate::command::runtime::protocol::{
    RuntimeCommand, SessionEvent, ToolImmediateFailure, ToolImmediateSuccess,
};
use crate::command::runtime::types::{EnqueueMessageAck, RuntimeSemaphores};
use crate::command::session_message::{map_input_parts, SessionCommandInputPart};
use crate::error::{CoreError, CoreResult};
use crate::event::{
    BackgroundJobCompletedPayload, BackgroundJobFailedPayload, BackgroundJobStartedPayload,
    CoreEvent, SessionContextPayload, SessionMessageStartedPayload, SessionPartUpdatedPayload,
    SessionRunCancelledPayload, SessionRunCompletedPayload,
};
use crate::llm::{LlmProvider, LlmStreamEvent, LlmTool, LlmToolSet};
use crate::message::message::MessageStorage;
use crate::message::{
    Message, PartBase, ToolPart, ToolState, ToolStateCompleted, ToolStateError, ToolStateRunning,
    ToolStateTimeCompleted, ToolStateTimeRange, ToolStateTimeStart,
};
use crate::session::SessionManager;
use crate::tool::ToolRegistry;
use crate::utils::time::now_secs;
use crate::workspace::WorkspaceInstance;

const MAX_TRACKED_CANCELLED_RUNS: usize = 512;

#[derive(Clone)]
pub struct SessionRuntimeHandle {
    session_id: String,
    event_tx: mpsc::UnboundedSender<SessionEvent>,
}

impl SessionRuntimeHandle {
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn is_closed(&self) -> bool {
        self.event_tx.is_closed()
    }

    pub async fn enqueue_user_message(
        &self,
        parts: Vec<SessionCommandInputPart>,
    ) -> CoreResult<EnqueueMessageAck> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.event_tx
            .send(SessionEvent::UserMessage {
                parts,
                reply: reply_tx,
            })
            .map_err(|_| CoreError::Internal("session runtime stopped".to_string()))?;
        reply_rx
            .await
            .map_err(|_| CoreError::Internal("session runtime dropped response".to_string()))?
    }
}

pub fn spawn_session_runtime(
    session_id: String,
    workspace: WorkspaceInstance,
    sessions: Arc<SessionManager>,
    llm: Arc<dyn LlmProvider>,
    bus: Bus,
    semaphores: RuntimeSemaphores,
) -> SessionRuntimeHandle {
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (command_tx, command_rx) = mpsc::unbounded_channel();

    spawn_runtime_executor(llm, semaphores, command_rx, event_tx.clone());

    let actor = SessionRuntimeActor::new(
        session_id.clone(),
        workspace,
        sessions,
        bus,
        event_tx.clone(),
        event_rx,
        command_tx,
    );

    tokio::spawn(async move {
        actor.run().await;
    });

    SessionRuntimeHandle {
        session_id,
        event_tx,
    }
}

struct SessionRuntimeActor {
    session_id: String,
    workspace: WorkspaceInstance,
    sessions: Arc<SessionManager>,
    bus: Bus,
    event_tx: mpsc::UnboundedSender<SessionEvent>,
    event_rx: mpsc::UnboundedReceiver<SessionEvent>,
    command_tx: mpsc::UnboundedSender<RuntimeCommand>,
    inflight: Option<RunState>,
    queued_followups: usize,
    tool_calls: HashMap<String, ToolCallRecord>,
    jobs: HashMap<String, JobInfo>,
    cancelled_runs: HashSet<String>,
    cancelled_runs_order: VecDeque<String>,
}

struct RunState {
    run_id: String,
    assistant_message: Message,
    processor: StreamProcessor,
    tools: LlmToolSet,
    cancel_token: CancellationToken,
}

#[derive(Clone)]
struct ToolCallRecord {
    session_id: String,
    run_id: String,
    message_id: String,
    part_id: String,
    tool_call_id: String,
    tool_name: String,
    input: Map<String, Value>,
    started_at: u64,
}

struct JobInfo {
    job_id: String,
    tool_call_id: String,
    tool_name: String,
    spawned_in_run: String,
    status: JobStatus,
    integrated: bool,
}

enum JobStatus {
    Running,
    Completed,
    Failed,
}

impl SessionRuntimeActor {
    fn new(
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
            tool_calls: HashMap::new(),
            jobs: HashMap::new(),
            cancelled_runs: HashSet::new(),
            cancelled_runs_order: VecDeque::new(),
        }
    }

    async fn run(mut self) {
        while let Some(event) = self.event_rx.recv().await {
            let result = match event {
                SessionEvent::UserMessage { parts, reply } => {
                    let result = self.handle_user_message(parts).await;
                    let _ = reply.send(result);
                    Ok(())
                }
                SessionEvent::LlmStreamPart { run_id, part } => {
                    self.handle_llm_stream_part(&run_id, part).await
                }
                SessionEvent::LlmFinished { run_id } => self.handle_llm_finished(&run_id).await,
                SessionEvent::LlmFailed {
                    run_id,
                    error,
                    cancelled,
                } => self.handle_llm_failed(&run_id, &error, cancelled).await,
                SessionEvent::ToolImmediateSuccess(payload) => {
                    self.handle_tool_immediate_success(payload).await
                }
                SessionEvent::ToolImmediateFailure(payload) => {
                    self.handle_tool_immediate_failure(payload).await
                }
                SessionEvent::ToolAsyncSpawned {
                    run_id,
                    tool_call_id,
                    tool_name,
                    job_id,
                } => {
                    self.handle_tool_async_spawned(run_id, tool_call_id, tool_name, job_id)
                        .await
                }
                SessionEvent::ToolAsyncCompleted { job_id, output } => {
                    self.handle_tool_async_completed(&job_id, output).await
                }
                SessionEvent::ToolAsyncFailed { job_id, error } => {
                    self.handle_tool_async_failed(&job_id, error).await
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
        self.cancel_current_run("superseded_by_user");
        self.queued_followups = 0;

        let mut user_message = Message::from_parts(&self.session_id, "user", Vec::new());
        user_message.parts = map_input_parts(parts, &self.session_id, &user_message.info.id);
        MessageStorage::store(&self.workspace.storage, &user_message).await?;

        let run_id = self.start_run(Some(user_message), Trigger::User).await?;

        Ok(EnqueueMessageAck {
            run_id,
            accepted_at: now_secs(),
        })
    }

    async fn start_run(
        &mut self,
        user_message: Option<Message>,
        _trigger: Trigger,
    ) -> CoreResult<String> {
        if self.inflight.is_some() {
            return Err(CoreError::Internal(
                "attempted to start run while llm already inflight".to_string(),
            ));
        }

        let run_id = Uuid::now_v7().to_string();
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
            processor: StreamProcessor::new(),
            tools,
            cancel_token,
        });

        Ok(run_id)
    }

    async fn handle_llm_stream_part(
        &mut self,
        run_id: &str,
        part: LlmStreamEvent,
    ) -> CoreResult<()> {
        let Some(run) = self.inflight.as_mut() else {
            return Ok(());
        };
        if run.run_id != run_id {
            return Ok(());
        }

        let context = StorePartContext::new(
            &self.workspace.storage,
            &self.bus,
            &run.run_id,
            &self.session_id,
            &run.assistant_message.info.id,
        );

        let tool_call = match &part {
            LlmStreamEvent::ToolCall {
                tool_call_id,
                tool_name,
                input,
            } => Some((tool_call_id.clone(), tool_name.clone(), input.clone())),
            _ => None,
        };

        run.processor.on_part(part, &context).await?;

        if let Some((tool_call_id, tool_name, input)) = tool_call {
            if let Some(tool_part) = run.processor.tool_call(&tool_call_id) {
                let (input_map, started_at) = running_input_and_start(&tool_part.state, now_secs());
                let record = ToolCallRecord {
                    session_id: self.session_id.clone(),
                    run_id: run_id.to_string(),
                    message_id: tool_part.base.message_id.clone(),
                    part_id: tool_part.base.id.clone(),
                    tool_call_id: tool_call_id.clone(),
                    tool_name: tool_name.clone(),
                    input: input_map,
                    started_at,
                };
                self.tool_calls.insert(tool_call_id.clone(), record);
            }
            self.dispatch_tool_call(run_id.to_string(), tool_call_id, tool_name, input)
                .await;
        }

        Ok(())
    }

    async fn dispatch_tool_call(
        &self,
        run_id: String,
        tool_call_id: String,
        tool_name: String,
        input: Value,
    ) {
        let Some(run) = self.inflight.as_ref() else {
            return;
        };

        let tool = run.tools.get(&tool_name).cloned();
        let is_async = is_async_tool_name(&tool_name);

        if self
            .command_tx
            .send(RuntimeCommand::CallTool {
                run_id,
                tool_call_id: tool_call_id.clone(),
                tool_name: tool_name.clone(),
                input,
                tool,
                is_async,
            })
            .is_err()
        {
            let _ = self
                .event_tx
                .send(SessionEvent::ToolImmediateFailure(ToolImmediateFailure {
                    run_id: run.run_id.clone(),
                    tool_call_id,
                    error: serde_json::json!({
                        "error": format!("runtime executor stopped before calling tool {tool_name}")
                    }),
                }));
        }
    }

    async fn handle_tool_immediate_success(
        &mut self,
        payload: ToolImmediateSuccess,
    ) -> CoreResult<()> {
        if self.cancelled_runs.contains(&payload.run_id) {
            return Ok(());
        }

        let Some(record) = self.tool_calls.get(&payload.tool_call_id).cloned() else {
            return Ok(());
        };
        if record.run_id != payload.run_id {
            return Ok(());
        }

        self.write_tool_completed(&record, payload.output).await?;
        self.schedule_followup().await?;
        Ok(())
    }

    async fn handle_tool_immediate_failure(
        &mut self,
        payload: ToolImmediateFailure,
    ) -> CoreResult<()> {
        if self.cancelled_runs.contains(&payload.run_id) {
            return Ok(());
        }

        let Some(record) = self.tool_calls.get(&payload.tool_call_id).cloned() else {
            return Ok(());
        };
        if record.run_id != payload.run_id {
            return Ok(());
        }

        self.write_tool_error(&record, payload.error).await?;
        self.schedule_followup().await?;
        Ok(())
    }

    async fn handle_tool_async_spawned(
        &mut self,
        run_id: String,
        tool_call_id: String,
        tool_name: String,
        job_id: String,
    ) -> CoreResult<()> {
        let Some(record) = self.tool_calls.get(&tool_call_id).cloned() else {
            return Ok(());
        };

        self.jobs.insert(
            job_id.clone(),
            JobInfo {
                job_id: job_id.clone(),
                tool_call_id: tool_call_id.clone(),
                tool_name: tool_name.clone(),
                spawned_in_run: run_id.clone(),
                status: JobStatus::Running,
                integrated: false,
            },
        );

        self.write_tool_running_metadata(&record, &job_id).await?;

        let _ = self.bus.publish(CoreEvent::BackgroundJobStarted(
            BackgroundJobStartedPayload {
                session_id: self.session_id.clone(),
                run_id,
                tool_call_id,
                tool_name,
                job_id,
            },
        ));

        Ok(())
    }

    async fn handle_tool_async_completed(&mut self, job_id: &str, output: Value) -> CoreResult<()> {
        let Some(job) = self.jobs.get_mut(job_id) else {
            return Ok(());
        };
        if job.integrated {
            return Ok(());
        }
        job.integrated = true;
        job.status = JobStatus::Completed;

        let tool_call_id = job.tool_call_id.clone();
        let tool_name = job.tool_name.clone();
        let spawned_in_run = job.spawned_in_run.clone();
        let job_id = job.job_id.clone();
        let Some(record) = self.tool_calls.get(&tool_call_id).cloned() else {
            return Ok(());
        };

        self.write_tool_completed(&record, output).await?;

        let _ = self.bus.publish(CoreEvent::BackgroundJobCompleted(
            BackgroundJobCompletedPayload {
                session_id: self.session_id.clone(),
                run_id: spawned_in_run,
                tool_call_id,
                tool_name,
                job_id,
            },
        ));

        self.schedule_followup().await?;
        Ok(())
    }

    async fn handle_tool_async_failed(&mut self, job_id: &str, error: Value) -> CoreResult<()> {
        let Some(job) = self.jobs.get_mut(job_id) else {
            return Ok(());
        };
        if job.integrated {
            return Ok(());
        }
        job.integrated = true;
        job.status = JobStatus::Failed;

        let tool_call_id = job.tool_call_id.clone();
        let tool_name = job.tool_name.clone();
        let spawned_in_run = job.spawned_in_run.clone();
        let job_id = job.job_id.clone();
        let Some(record) = self.tool_calls.get(&tool_call_id).cloned() else {
            return Ok(());
        };

        self.write_tool_error(&record, error.clone()).await?;

        let _ = self
            .bus
            .publish(CoreEvent::BackgroundJobFailed(BackgroundJobFailedPayload {
                session_id: self.session_id.clone(),
                run_id: spawned_in_run,
                tool_call_id,
                tool_name,
                job_id,
                error: value_to_string(&error),
            }));

        self.schedule_followup().await?;
        Ok(())
    }

    async fn handle_llm_finished(&mut self, run_id: &str) -> CoreResult<()> {
        let Some(run) = self.inflight.as_ref() else {
            return Ok(());
        };
        if run.run_id != run_id {
            return Ok(());
        }

        let mut run = self.inflight.take().expect("inflight exists");
        MessageStorage::touch_info(&self.workspace.storage, &mut run.assistant_message.info)
            .await?;

        let _ = self
            .bus
            .publish(CoreEvent::SessionRunCompleted(SessionRunCompletedPayload {
                session_id: self.session_id.clone(),
                run_id: run_id.to_string(),
            }));

        if self.queued_followups > 0 {
            self.queued_followups -= 1;
            let _ = self.start_run(None, Trigger::Followup).await?;
        }

        Ok(())
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

        if self.queued_followups > 0 {
            self.queued_followups -= 1;
            let _ = self.start_run(None, Trigger::Followup).await?;
        }

        Ok(())
    }

    async fn schedule_followup(&mut self) -> CoreResult<()> {
        if self.inflight.is_some() {
            self.queued_followups = self.queued_followups.saturating_add(1);
            return Ok(());
        }
        let _ = self.start_run(None, Trigger::Followup).await?;
        Ok(())
    }

    fn cancel_current_run(&mut self, reason: &str) {
        let Some(run) = self.inflight.take() else {
            return;
        };
        self.remember_cancelled_run(run.run_id.clone());
        run.cancel_token.cancel();
        let _ = self
            .bus
            .publish(CoreEvent::SessionRunCancelled(SessionRunCancelledPayload {
                session_id: self.session_id.clone(),
                run_id: run.run_id,
                reason: reason.to_string(),
            }));
    }

    fn remember_cancelled_run(&mut self, run_id: String) {
        if !self.cancelled_runs.insert(run_id.clone()) {
            return;
        }

        self.cancelled_runs_order.push_back(run_id);
        while self.cancelled_runs_order.len() > MAX_TRACKED_CANCELLED_RUNS {
            if let Some(evicted) = self.cancelled_runs_order.pop_front() {
                self.cancelled_runs.remove(&evicted);
            }
        }
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

    async fn write_tool_running_metadata(
        &self,
        record: &ToolCallRecord,
        job_id: &str,
    ) -> CoreResult<()> {
        let mut metadata = Map::new();
        metadata.insert("job_id".to_string(), Value::String(job_id.to_string()));
        metadata.insert("status".to_string(), Value::String("running".to_string()));

        let part = ToolPart {
            base: PartBase {
                id: record.part_id.clone(),
                session_id: record.session_id.clone(),
                message_id: record.message_id.clone(),
            },
            call_id: record.tool_call_id.clone(),
            tool: record.tool_name.clone(),
            state: ToolState::Running(ToolStateRunning {
                input: record.input.clone(),
                title: None,
                metadata: Some(metadata),
                time: ToolStateTimeStart {
                    start: record.started_at,
                },
            }),
            metadata: None,
        };
        MessageStorage::store_part(
            &self.workspace.storage,
            &crate::message::MessagePart::Tool(part.clone()),
        )
        .await?;
        let _ = self
            .bus
            .publish(CoreEvent::SessionPartUpdated(SessionPartUpdatedPayload {
                session_id: record.session_id.clone(),
                run_id: record.run_id.clone(),
                message_id: record.message_id.clone(),
                part_id: record.part_id.clone(),
                part: crate::message::MessagePart::Tool(part),
            }));
        Ok(())
    }

    async fn write_tool_completed(&self, record: &ToolCallRecord, output: Value) -> CoreResult<()> {
        let end = now_secs();
        let part = ToolPart {
            base: PartBase {
                id: record.part_id.clone(),
                session_id: record.session_id.clone(),
                message_id: record.message_id.clone(),
            },
            call_id: record.tool_call_id.clone(),
            tool: record.tool_name.clone(),
            state: ToolState::Completed(ToolStateCompleted {
                input: record.input.clone(),
                output: value_to_string(&output),
                title: record.tool_name.clone(),
                metadata: Map::new(),
                time: ToolStateTimeCompleted {
                    start: record.started_at,
                    end,
                    compacted: None,
                },
                attachments: None,
            }),
            metadata: None,
        };
        MessageStorage::store_part(
            &self.workspace.storage,
            &crate::message::MessagePart::Tool(part.clone()),
        )
        .await?;
        let _ = self
            .bus
            .publish(CoreEvent::SessionPartUpdated(SessionPartUpdatedPayload {
                session_id: record.session_id.clone(),
                run_id: record.run_id.clone(),
                message_id: record.message_id.clone(),
                part_id: record.part_id.clone(),
                part: crate::message::MessagePart::Tool(part),
            }));
        Ok(())
    }

    async fn write_tool_error(&self, record: &ToolCallRecord, error: Value) -> CoreResult<()> {
        let end = now_secs();
        let part = ToolPart {
            base: PartBase {
                id: record.part_id.clone(),
                session_id: record.session_id.clone(),
                message_id: record.message_id.clone(),
            },
            call_id: record.tool_call_id.clone(),
            tool: record.tool_name.clone(),
            state: ToolState::Error(ToolStateError {
                input: record.input.clone(),
                error: value_to_string(&error),
                metadata: None,
                time: ToolStateTimeRange {
                    start: record.started_at,
                    end,
                },
            }),
            metadata: None,
        };
        MessageStorage::store_part(
            &self.workspace.storage,
            &crate::message::MessagePart::Tool(part.clone()),
        )
        .await?;
        let _ = self
            .bus
            .publish(CoreEvent::SessionPartUpdated(SessionPartUpdatedPayload {
                session_id: record.session_id.clone(),
                run_id: record.run_id.clone(),
                message_id: record.message_id.clone(),
                part_id: record.part_id.clone(),
                part: crate::message::MessagePart::Tool(part),
            }));
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Trigger {
    User,
    Followup,
}

fn strip_tool_execute(tools: &LlmToolSet) -> LlmToolSet {
    tools
        .iter()
        .map(|(name, tool)| {
            (
                name.clone(),
                LlmTool {
                    description: tool.description.clone(),
                    input_schema: tool.input_schema.clone(),
                    execute: None,
                },
            )
        })
        .collect()
}

fn is_async_tool_name(name: &str) -> bool {
    name == "subagent_run" || name == "agent_execute-agent"
}

fn running_input_and_start(state: &ToolState, fallback_start: u64) -> (Map<String, Value>, u64) {
    match state {
        ToolState::Pending(state) => (state.input.clone(), fallback_start),
        ToolState::Running(state) => (state.input.clone(), state.time.start),
        ToolState::Completed(state) => (state.input.clone(), state.time.start),
        ToolState::Error(state) => (state.input.clone(), state.time.start),
    }
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| "null".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;
    use tempfile::tempdir;
    use tokio::sync::mpsc;

    use super::*;
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
}
