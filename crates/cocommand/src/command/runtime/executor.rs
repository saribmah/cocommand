mod llm;
mod processor;
#[cfg(test)]
mod tests;
mod tool;
mod tool_processor;

use std::sync::Arc;

use tokio::sync::mpsc;

use self::llm::spawn_llm_execution;
use self::tool::spawn_tool_execution;
use crate::bus::Bus;
use crate::command::runtime::protocol::{RuntimeCommand, SessionEvent};
use crate::command::runtime::types::RuntimeSemaphores;
use crate::llm::LlmProvider;
use crate::storage::SharedStorage;

pub(crate) fn spawn_runtime_executor(
    llm: Arc<dyn LlmProvider>,
    semaphores: RuntimeSemaphores,
    storage: SharedStorage,
    bus: Bus,
    session_id: String,
    mut command_rx: mpsc::UnboundedReceiver<RuntimeCommand>,
    event_tx: mpsc::UnboundedSender<SessionEvent>,
) {
    tokio::spawn(async move {
        while let Some(command) = command_rx.recv().await {
            match command {
                RuntimeCommand::CallLlm {
                    run_id,
                    assistant_message_id,
                    messages,
                    tools,
                    cancel_token,
                } => {
                    spawn_llm_execution(
                        llm.clone(),
                        semaphores.clone(),
                        storage.clone(),
                        bus.clone(),
                        session_id.clone(),
                        event_tx.clone(),
                        run_id,
                        assistant_message_id,
                        messages,
                        tools,
                        cancel_token,
                    );
                }
                RuntimeCommand::CallTool {
                    context,
                    input,
                    tool,
                } => {
                    spawn_tool_execution(
                        storage.clone(),
                        bus.clone(),
                        event_tx.clone(),
                        semaphores.clone(),
                        context,
                        input,
                        tool,
                    );
                }
            }
        }
    });
}
