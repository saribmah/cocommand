use futures_util::future::join_all;
use serde_json::json;
use tokio::sync::mpsc;

use crate::bus::Bus;
use crate::command::runtime::protocol::{SessionEvent, ToolBatchCall, ToolBatchResult};
use crate::command::runtime::types::RuntimeSemaphores;
use crate::storage::SharedStorage;

use super::tool_processor::{value_to_string, ToolProcessor};

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_tool_batch_execution(
    storage: SharedStorage,
    bus: Bus,
    event_tx: mpsc::UnboundedSender<SessionEvent>,
    semaphores: RuntimeSemaphores,
    run_id: String,
    calls: Vec<ToolBatchCall>,
) {
    tokio::spawn(async move {
        let futures = calls
            .into_iter()
            .map(|call| execute_tool_call(storage.clone(), bus.clone(), semaphores.clone(), call));
        let results = join_all(futures).await;

        let _ = event_tx.send(SessionEvent::ToolBatchFinished { run_id, results });
    });
}

async fn execute_tool_call(
    storage: SharedStorage,
    bus: Bus,
    semaphores: RuntimeSemaphores,
    call: ToolBatchCall,
) -> ToolBatchResult {
    let ToolBatchCall {
        context,
        input,
        tool,
    } = call;
    let tool_call_id = context.tool_call_id.clone();
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
            return ToolBatchResult {
                tool_call_id,
                success: false,
                error: Some(value_to_string(&error)),
            };
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
            return ToolBatchResult {
                tool_call_id,
                success: false,
                error: Some(value_to_string(&error)),
            };
        }
    };

    let result = execute(input).await;
    match result {
        Ok(output) => {
            let output_value = match serde_json::to_value(output) {
                Ok(value) => value,
                Err(error) => {
                    let error_value = json!({
                        "error": format!("failed to serialize tool output envelope: {error}")
                    });
                    if let Err(write_error) =
                        processor.apply_error(&context, error_value.clone()).await
                    {
                        tracing::warn!(
                            "failed to persist tool serialization error for {}: {}",
                            context.tool_call_id,
                            write_error
                        );
                    }
                    return ToolBatchResult {
                        tool_call_id,
                        success: false,
                        error: Some(value_to_string(&error_value)),
                    };
                }
            };
            if let Err(write_error) = processor.apply_completed(&context, output_value).await {
                tracing::warn!(
                    "failed to persist tool completion for {}: {}",
                    context.tool_call_id,
                    write_error
                );
            }
            ToolBatchResult {
                tool_call_id,
                success: true,
                error: None,
            }
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
            ToolBatchResult {
                tool_call_id,
                success: false,
                error: Some(error_str),
            }
        }
    }
}
