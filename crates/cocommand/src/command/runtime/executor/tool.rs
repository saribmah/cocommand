use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::bus::Bus;
use crate::command::runtime::protocol::{
    SessionEvent, ToolExecutionContext, ToolFailure, ToolSuccess,
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
                let _ = event_tx.send(SessionEvent::ToolFailure(ToolFailure {
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
                let _ = event_tx.send(SessionEvent::ToolFailure(ToolFailure {
                    run_id: context.run_id.clone(),
                    tool_call_id: context.tool_call_id.clone(),
                    error: value_to_string(&error),
                }));
                return;
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
                        let _ = event_tx.send(SessionEvent::ToolFailure(ToolFailure {
                            run_id: context.run_id.clone(),
                            tool_call_id: context.tool_call_id.clone(),
                            error: value_to_string(&error_value),
                        }));
                        return;
                    }
                };
                if let Err(write_error) = processor.apply_completed(&context, output_value).await {
                    tracing::warn!(
                        "failed to persist tool completion for {}: {}",
                        context.tool_call_id,
                        write_error
                    );
                }
                let _ = event_tx.send(SessionEvent::ToolSuccess(ToolSuccess {
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
                let _ = event_tx.send(SessionEvent::ToolFailure(ToolFailure {
                    run_id: context.run_id.clone(),
                    tool_call_id: context.tool_call_id.clone(),
                    error: error_str,
                }));
            }
        }
    });
}
