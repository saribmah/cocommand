mod llm;
#[cfg(test)]
mod tests;
mod tool;

use std::sync::Arc;

use tokio::sync::mpsc;

use self::llm::spawn_llm_execution;
use self::tool::spawn_tool_execution;
use crate::command::runtime::protocol::{RuntimeCommand, SessionEvent};
use crate::command::runtime::types::RuntimeSemaphores;
use crate::llm::LlmProvider;

pub(crate) fn spawn_runtime_executor(
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
