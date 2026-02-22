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
const DEFAULT_MAX_CONCURRENT_ASYNC_JOBS: usize = 64;

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
                jobs: Arc::new(tokio::sync::Semaphore::new(read_limit(
                    "COCOMMAND_MAX_CONCURRENT_ASYNC_JOBS",
                    DEFAULT_MAX_CONCURRENT_ASYNC_JOBS,
                ))),
            },
        }
    }

    pub async fn get_or_create(&self, session_id: &str) -> SessionRuntimeHandle {
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
            self.sessions.clone(),
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
