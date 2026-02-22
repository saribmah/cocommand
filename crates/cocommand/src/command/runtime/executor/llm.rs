use std::sync::Arc;

use tokio::sync::mpsc;

use super::processor::{StorePartContext, StreamProcessor};
use crate::bus::Bus;
use crate::command::runtime::protocol::SessionEvent;
use crate::command::runtime::types::RuntimeSemaphores;
use crate::llm::{LlmProvider, LlmStreamOptions, LlmToolSet};
use crate::message::Message;
use crate::storage::SharedStorage;

pub(super) fn spawn_llm_execution(
    llm: Arc<dyn LlmProvider>,
    semaphores: RuntimeSemaphores,
    storage: SharedStorage,
    bus: Bus,
    session_id: String,
    event_tx: mpsc::UnboundedSender<SessionEvent>,
    run_id: String,
    assistant_message_id: String,
    messages: Vec<Message>,
    tools: LlmToolSet,
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

        let stream = match stream {
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

        let mut processor = StreamProcessor::new();
        let context =
            StorePartContext::new(&storage, &bus, &run_id, &session_id, &assistant_message_id);
        let result = processor.process(stream, &context).await;
        match result {
            Ok(()) => {
                let _ = event_tx.send(SessionEvent::LlmFinished {
                    run_id,
                    parts: processor.mapped_parts().to_vec(),
                });
            }
            Err(error) => {
                let _ = event_tx.send(SessionEvent::LlmFailed {
                    run_id,
                    error: error.to_string(),
                    cancelled: cancel_token.is_cancelled(),
                });
            }
        }
    });
}
