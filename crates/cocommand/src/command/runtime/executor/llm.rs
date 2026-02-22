use std::sync::Arc;

use tokio::sync::mpsc;
use tokio_stream::StreamExt;

use crate::command::runtime::protocol::SessionEvent;
use crate::command::runtime::types::RuntimeSemaphores;
use crate::llm::{LlmProvider, LlmStreamOptions, LlmToolSet};
use crate::message::Message;

pub(super) fn spawn_llm_execution(
    llm: Arc<dyn LlmProvider>,
    semaphores: RuntimeSemaphores,
    event_tx: mpsc::UnboundedSender<SessionEvent>,
    run_id: String,
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

        let mut stream = match stream {
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

        while let Some(part) = stream.next().await {
            if event_tx
                .send(SessionEvent::LlmStreamPart {
                    run_id: run_id.clone(),
                    part,
                })
                .is_err()
            {
                return;
            }
        }

        let _ = event_tx.send(SessionEvent::LlmFinished { run_id });
    });
}
