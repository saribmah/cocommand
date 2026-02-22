use serde_json::{json, Value};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::command::runtime::protocol::{SessionEvent, ToolImmediateFailure, ToolImmediateSuccess};
use crate::command::runtime::types::RuntimeSemaphores;
use crate::llm::LlmTool;

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_tool_execution(
    event_tx: mpsc::UnboundedSender<SessionEvent>,
    semaphores: RuntimeSemaphores,
    run_id: String,
    tool_call_id: String,
    tool_name: String,
    input: Value,
    tool: Option<LlmTool>,
    is_async: bool,
) {
    tokio::spawn(async move {
        let _tool_permit = match semaphores.tool.acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => {
                let _ = event_tx.send(SessionEvent::ToolImmediateFailure(ToolImmediateFailure {
                    run_id,
                    tool_call_id,
                    error: json!({ "error": "tool semaphore closed" }),
                }));
                return;
            }
        };

        let execute = match tool.and_then(|tool| tool.execute.clone()) {
            Some(execute) => execute,
            None => {
                let _ = event_tx.send(SessionEvent::ToolImmediateFailure(ToolImmediateFailure {
                    run_id,
                    tool_call_id,
                    error: json!({ "error": format!("tool not available: {tool_name}") }),
                }));
                return;
            }
        };

        if is_async {
            let job_id = Uuid::now_v7().to_string();
            let job_permit = match semaphores.jobs.acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => {
                    let _ =
                        event_tx.send(SessionEvent::ToolImmediateFailure(ToolImmediateFailure {
                            run_id,
                            tool_call_id,
                            error: json!({ "error": "async job semaphore closed" }),
                        }));
                    return;
                }
            };

            let _ = event_tx.send(SessionEvent::ToolAsyncSpawned {
                run_id: run_id.clone(),
                tool_call_id: tool_call_id.clone(),
                tool_name: tool_name.clone(),
                job_id: job_id.clone(),
            });

            let event_tx_job = event_tx.clone();
            tokio::spawn(async move {
                let _job_permit = job_permit;
                let result = execute(input).await;
                match result {
                    Ok(output) => {
                        let _ =
                            event_tx_job.send(SessionEvent::ToolAsyncCompleted { job_id, output });
                    }
                    Err(error) => {
                        let _ = event_tx_job.send(SessionEvent::ToolAsyncFailed { job_id, error });
                    }
                }
            });
            return;
        }

        let result = execute(input).await;
        match result {
            Ok(output) => {
                let _ = event_tx.send(SessionEvent::ToolImmediateSuccess(ToolImmediateSuccess {
                    run_id,
                    tool_call_id,
                    output,
                }));
            }
            Err(error) => {
                let _ = event_tx.send(SessionEvent::ToolImmediateFailure(ToolImmediateFailure {
                    run_id,
                    tool_call_id,
                    error,
                }));
            }
        }
    });
}
