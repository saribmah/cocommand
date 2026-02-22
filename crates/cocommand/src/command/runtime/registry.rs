use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::bus::Bus;
use crate::command::runtime::handle::{spawn_session_runtime, SessionRuntimeHandle};
use crate::command::runtime::types::RuntimeSemaphores;
use crate::llm::LlmProvider;
use crate::session::SessionManager;
use crate::workspace::WorkspaceInstance;

const DEFAULT_MAX_CONCURRENT_LLM: usize = 8;
const DEFAULT_MAX_CONCURRENT_TOOLS: usize = 32;

#[derive(Clone)]
pub struct SessionRuntimeRegistry {
    runtimes: Arc<Mutex<HashMap<String, SessionRuntimeHandle>>>,
    workspace: WorkspaceInstance,
    sessions: Arc<SessionManager>,
    llm: Arc<dyn LlmProvider>,
    bus: Bus,
    semaphores: RuntimeSemaphores,
}

impl SessionRuntimeRegistry {
    pub fn new(
        workspace: WorkspaceInstance,
        sessions: Arc<SessionManager>,
        llm: Arc<dyn LlmProvider>,
        bus: Bus,
    ) -> Self {
        Self {
            runtimes: Arc::new(Mutex::new(HashMap::new())),
            workspace,
            sessions,
            llm,
            bus,
            semaphores: RuntimeSemaphores {
                llm: Arc::new(tokio::sync::Semaphore::new(read_limit(
                    "COCOMMAND_MAX_CONCURRENT_LLM",
                    DEFAULT_MAX_CONCURRENT_LLM,
                ))),
                tool: Arc::new(tokio::sync::Semaphore::new(read_limit(
                    "COCOMMAND_MAX_CONCURRENT_TOOLS",
                    DEFAULT_MAX_CONCURRENT_TOOLS,
                ))),
            },
        }
    }

    pub async fn get_or_create(&self, session_id: &str) -> SessionRuntimeHandle {
        self.get_or_create_with_sessions(session_id, self.sessions.clone())
            .await
    }

    pub async fn spawn_with_session_manager(
        &self,
        session_id: String,
        sessions: Arc<SessionManager>,
    ) -> SessionRuntimeHandle {
        self.get_or_create_with_sessions(&session_id, sessions)
            .await
    }

    async fn get_or_create_with_sessions(
        &self,
        session_id: &str,
        sessions: Arc<SessionManager>,
    ) -> SessionRuntimeHandle {
        let mut runtimes = self.runtimes.lock().await;
        if let Some(existing) = runtimes.get(session_id) {
            if !existing.is_closed() {
                return existing.clone();
            }
            runtimes.remove(session_id);
        }

        let handle = spawn_session_runtime(
            session_id.to_string(),
            self.workspace.clone(),
            sessions,
            self.llm.clone(),
            self.bus.clone(),
            self.semaphores.clone(),
        );
        runtimes.insert(session_id.to_string(), handle.clone());
        handle
    }
}

fn read_limit(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}
