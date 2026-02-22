use serde_json::{json, Value};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::bus::Bus;
use crate::command::runtime::protocol::{
    SessionEvent, ToolExecutionContext, ToolImmediateFailure, ToolImmediateSuccess,
};
use crate::command::runtime::types::RuntimeSemaphores;
use crate::llm::LlmTool;
use crate::storage::SharedStorage;

use super::tool_processor::{value_to_string, ToolProcessor};

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_tool_execution(
    storage: SharedStorage,
    bus: Bus,
    event_tx: mpsc::UnboundedSender<SessionEvent>,
    semaphores: RuntimeSemaphores,
    context: ToolExecutionContext,
    input: Value,
    tool: Option<LlmTool>,
    is_async: bool,
) {
    tokio::spawn(async move {
        let processor = ToolProcessor::new(storage, bus);

        let _tool_permit = match semaphores.tool.acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => {
                let error = json!({ "error": "tool semaphore closed" });
                if let Err(write_error) = processor.apply_error(&context, error.clone()).await {
                    tracing::warn!(
                        "failed to persist tool error for {}: {}",
                        context.tool_call_id,
                        write_error
                    );
                }
                let _ = event_tx.send(SessionEvent::ToolImmediateFailure(ToolImmediateFailure {
                    run_id: context.run_id.clone(),
                    tool_call_id: context.tool_call_id.clone(),
                    error: value_to_string(&error),
                }));
                return;
            }
        };

        let execute = match tool.and_then(|tool| tool.execute.clone()) {
            Some(execute) => execute,
            None => {
                let error = json!({
                    "error": format!("tool not available: {}", context.tool_name)
                });
                if let Err(write_error) = processor.apply_error(&context, error.clone()).await {
                    tracing::warn!(
                        "failed to persist tool error for {}: {}",
                        context.tool_call_id,
                        write_error
                    );
                }
                let _ = event_tx.send(SessionEvent::ToolImmediateFailure(ToolImmediateFailure {
                    run_id: context.run_id.clone(),
                    tool_call_id: context.tool_call_id.clone(),
                    error: value_to_string(&error),
                }));
                return;
            }
        };

        if is_async {
            let job_id = Uuid::now_v7().to_string();
            let job_permit = match semaphores.jobs.acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => {
                    let error = json!({ "error": "async job semaphore closed" });
                    if let Err(write_error) = processor.apply_error(&context, error.clone()).await {
                        tracing::warn!(
                            "failed to persist tool error for {}: {}",
                            context.tool_call_id,
                            write_error
                        );
                    }
                    let _ =
                        event_tx.send(SessionEvent::ToolImmediateFailure(ToolImmediateFailure {
                            run_id: context.run_id.clone(),
                            tool_call_id: context.tool_call_id.clone(),
                            error: value_to_string(&error),
                        }));
                    return;
                }
            };

            if let Err(write_error) = processor.apply_running_metadata(&context, &job_id).await {
                tracing::warn!(
                    "failed to persist async running state for {}: {}",
                    context.tool_call_id,
                    write_error
                );
            }

            let _ = event_tx.send(SessionEvent::ToolAsyncSpawned {
                run_id: context.run_id.clone(),
                tool_call_id: context.tool_call_id.clone(),
                tool_name: context.tool_name.clone(),
                job_id: job_id.clone(),
            });

            let event_tx_job = event_tx.clone();
            tokio::spawn(async move {
                let _job_permit = job_permit;
                let result = execute(input).await;
                match result {
                    Ok(output) => {
                        if let Err(write_error) = processor.apply_completed(&context, output).await
                        {
                            tracing::warn!(
                                "failed to persist async tool completion for {}: {}",
                                context.tool_call_id,
                                write_error
                            );
                        }
                        let _ = event_tx_job.send(SessionEvent::ToolAsyncCompleted { job_id });
                    }
                    Err(error) => {
                        let error_str = value_to_string(&error);
                        if let Err(write_error) = processor.apply_error(&context, error).await {
                            tracing::warn!(
                                "failed to persist async tool failure for {}: {}",
                                context.tool_call_id,
                                write_error
                            );
                        }
                        let _ = event_tx_job.send(SessionEvent::ToolAsyncFailed {
                            job_id,
                            error: error_str,
                        });
                    }
                }
            });
            return;
        }

        let result = execute(input).await;
        match result {
            Ok(output) => {
                if let Err(write_error) = processor.apply_completed(&context, output).await {
                    tracing::warn!(
                        "failed to persist tool completion for {}: {}",
                        context.tool_call_id,
                        write_error
                    );
                }
                let _ = event_tx.send(SessionEvent::ToolImmediateSuccess(ToolImmediateSuccess {
                    run_id: context.run_id.clone(),
                    tool_call_id: context.tool_call_id.clone(),
                }));
            }
            Err(error) => {
                let error_str = value_to_string(&error);
                if let Err(write_error) = processor.apply_error(&context, error).await {
                    tracing::warn!(
                        "failed to persist tool failure for {}: {}",
                        context.tool_call_id,
                        write_error
                    );
                }
                let _ = event_tx.send(SessionEvent::ToolImmediateFailure(ToolImmediateFailure {
                    run_id: context.run_id.clone(),
                    tool_call_id: context.tool_call_id.clone(),
                    error: error_str,
                }));
            }
        }
    });
}
