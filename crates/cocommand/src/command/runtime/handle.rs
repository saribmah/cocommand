use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};

use crate::bus::Bus;
use crate::command::runtime::actor::SessionRuntimeActor;
use crate::command::runtime::executor::spawn_runtime_executor;
use crate::command::runtime::protocol::SessionEvent;
use crate::command::runtime::types::{EnqueueMessageAck, RuntimeSemaphores};
use crate::command::session_message::SessionCommandInputPart;
use crate::error::{CoreError, CoreResult};
use crate::llm::LlmProvider;
use crate::session::SessionManager;
use crate::workspace::WorkspaceInstance;

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

    spawn_runtime_executor(
        llm,
        semaphores,
        workspace.storage.clone(),
        bus.clone(),
        session_id.clone(),
        command_rx,
        event_tx.clone(),
    );

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
