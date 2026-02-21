use std::sync::Arc;

use tokio::sync::Semaphore;

#[derive(Debug, Clone)]
pub struct EnqueueMessageAck {
    pub run_id: String,
    pub accepted_at: u64,
}

#[derive(Clone)]
pub struct RuntimeSemaphores {
    pub llm: Arc<Semaphore>,
    pub tool: Arc<Semaphore>,
    pub jobs: Arc<Semaphore>,
}
