use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{Map, Value};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::bus::Bus;
use crate::command::processor::{StorePartContext, StreamProcessor};
use crate::command::runtime::tool_worker::{
    spawn_tool_execution, ToolImmediateFailure, ToolImmediateSuccess,
};
use crate::command::runtime::types::{EnqueueMessageAck, RuntimeSemaphores};
use crate::command::session_message::{map_input_parts, SessionCommandInputPart};
use crate::error::{CoreError, CoreResult};
use crate::event::{
    BackgroundJobCompletedPayload, BackgroundJobFailedPayload, BackgroundJobStartedPayload,
    CoreEvent, SessionContextPayload, SessionMessageStartedPayload, SessionPartUpdatedPayload,
    SessionRunCancelledPayload, SessionRunCompletedPayload,
};
use crate::llm::{LlmProvider, LlmStreamEvent, LlmStreamOptions, LlmTool, LlmToolSet};
use crate::message::message::MessageStorage;
use crate::message::{
    Message, PartBase, ToolPart, ToolState, ToolStateCompleted, ToolStateError, ToolStateRunning,
    ToolStateTimeCompleted, ToolStateTimeRange, ToolStateTimeStart,
};
use crate::session::SessionManager;
use crate::tool::ToolRegistry;
use crate::utils::time::now_secs;
use crate::workspace::WorkspaceInstance;

#[derive(Clone)]
pub struct SessionRuntimeHandle {
    session_id: String,
    tx: mpsc::UnboundedSender<ActorCommand>,
}

impl SessionRuntimeHandle {
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }

    pub async fn enqueue_user_message(
        &self,
        parts: Vec<SessionCommandInputPart>,
    ) -> CoreResult<EnqueueMessageAck> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(ActorCommand::UserMessage {
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
    let (tx, rx) = mpsc::unbounded_channel();
    let actor = SessionRuntimeActor::new(
        session_id.clone(),
        workspace,
        sessions,
        llm,
        bus,
        semaphores,
        tx.clone(),
        rx,
    );
    tokio::spawn(async move {
        actor.run().await;
    });
    SessionRuntimeHandle { session_id, tx }
}

pub(crate) enum ActorCommand {
    UserMessage {
        parts: Vec<SessionCommandInputPart>,
        reply: oneshot::Sender<CoreResult<EnqueueMessageAck>>,
    },
    LlmStreamPart {
        run_id: String,
        part: LlmStreamEvent,
    },
    LlmFinished {
        run_id: String,
    },
    LlmFailed {
        run_id: String,
        error: String,
        cancelled: bool,
    },
    ToolImmediateSuccess(ToolImmediateSuccess),
    ToolImmediateFailure(ToolImmediateFailure),
    ToolAsyncSpawned {
        run_id: String,
        tool_call_id: String,
        tool_name: String,
        job_id: String,
    },
    ToolAsyncCompleted {
        job_id: String,
        output: Value,
    },
    ToolAsyncFailed {
        job_id: String,
        error: Value,
    },
}

struct SessionRuntimeActor {
    session_id: String,
    workspace: WorkspaceInstance,
    sessions: Arc<SessionManager>,
    llm: Arc<dyn LlmProvider>,
    bus: Bus,
    semaphores: RuntimeSemaphores,
    tx: mpsc::UnboundedSender<ActorCommand>,
    rx: mpsc::UnboundedReceiver<ActorCommand>,
    inflight: Option<RunState>,
    queued_followups: usize,
    tool_calls: HashMap<String, ToolCallRecord>,
    jobs: HashMap<String, JobInfo>,
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
        llm: Arc<dyn LlmProvider>,
        bus: Bus,
        semaphores: RuntimeSemaphores,
        tx: mpsc::UnboundedSender<ActorCommand>,
        rx: mpsc::UnboundedReceiver<ActorCommand>,
    ) -> Self {
        Self {
            session_id,
            workspace,
            sessions,
            llm,
            bus,
            semaphores,
            tx,
            rx,
            inflight: None,
            queued_followups: 0,
            tool_calls: HashMap::new(),
            jobs: HashMap::new(),
        }
    }

    async fn run(mut self) {
        while let Some(command) = self.rx.recv().await {
            let result = match command {
                ActorCommand::UserMessage { parts, reply } => {
                    let result = self.handle_user_message(parts).await;
                    let _ = reply.send(result);
                    Ok(())
                }
                ActorCommand::LlmStreamPart { run_id, part } => {
                    self.handle_llm_stream_part(&run_id, part).await
                }
                ActorCommand::LlmFinished { run_id } => self.handle_llm_finished(&run_id).await,
                ActorCommand::LlmFailed {
                    run_id,
                    error,
                    cancelled,
                } => self.handle_llm_failed(&run_id, &error, cancelled).await,
                ActorCommand::ToolImmediateSuccess(payload) => {
                    self.handle_tool_immediate_success(payload).await
                }
                ActorCommand::ToolImmediateFailure(payload) => {
                    self.handle_tool_immediate_failure(payload).await
                }
                ActorCommand::ToolAsyncSpawned {
                    run_id,
                    tool_call_id,
                    tool_name,
                    job_id,
                } => {
                    self.handle_tool_async_spawned(run_id, tool_call_id, tool_name, job_id)
                        .await
                }
                ActorCommand::ToolAsyncCompleted { job_id, output } => {
                    self.handle_tool_async_completed(&job_id, output).await
                }
                ActorCommand::ToolAsyncFailed { job_id, error } => {
                    self.handle_tool_async_failed(&job_id, error).await
                }
            };

            if let Err(error) = result {
                tracing::warn!(
                    "session runtime actor {} command failed: {}",
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
        trigger: Trigger,
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
        self.spawn_llm_worker(
            run_id.clone(),
            messages,
            llm_tools,
            cancel_token.clone(),
            trigger,
        );

        self.inflight = Some(RunState {
            run_id: run_id.clone(),
            assistant_message,
            processor: StreamProcessor::new(),
            tools,
            cancel_token,
        });

        Ok(run_id)
    }

    fn spawn_llm_worker(
        &self,
        run_id: String,
        messages: Vec<Message>,
        tools: LlmToolSet,
        cancel_token: CancellationToken,
        _trigger: Trigger,
    ) {
        let tx = self.tx.clone();
        let llm = self.llm.clone();
        let llm_permits = self.semaphores.llm.clone();

        tokio::spawn(async move {
            let _permit = match llm_permits.acquire_owned().await {
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
                    let _ = tx.send(ActorCommand::LlmFailed {
                        run_id,
                        error: error.to_string(),
                        cancelled: cancel_token.is_cancelled(),
                    });
                    return;
                }
            };

            while let Some(part) = stream.next().await {
                if tx
                    .send(ActorCommand::LlmStreamPart {
                        run_id: run_id.clone(),
                        part,
                    })
                    .is_err()
                {
                    return;
                }
            }

            let _ = tx.send(ActorCommand::LlmFinished { run_id });
        });
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
        spawn_tool_execution(
            self.tx.clone(),
            self.semaphores.clone(),
            run_id,
            tool_call_id,
            tool_name,
            input,
            tool,
            is_async,
        );
    }

    async fn handle_tool_immediate_success(
        &mut self,
        payload: ToolImmediateSuccess,
    ) -> CoreResult<()> {
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
        run.cancel_token.cancel();
        let _ = self
            .bus
            .publish(CoreEvent::SessionRunCancelled(SessionRunCancelledPayload {
                session_id: self.session_id.clone(),
                run_id: run.run_id,
                reason: reason.to_string(),
            }));
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
