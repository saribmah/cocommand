use serde_json::{Map, Value};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::command::runtime::types::EnqueueMessageAck;
use crate::command::session_message::SessionCommandInputPart;
use crate::error::CoreResult;
use crate::llm::{LlmTool, LlmToolSet};
use crate::message::{Message, MessagePart};

pub enum SessionEvent {
    UserMessage {
        parts: Vec<SessionCommandInputPart>,
        reply: oneshot::Sender<CoreResult<EnqueueMessageAck>>,
    },
    LlmFinished {
        run_id: String,
        parts: Vec<MessagePart>,
    },
    LlmFailed {
        run_id: String,
        error: String,
        cancelled: bool,
    },
    ToolImmediateSuccess(ToolImmediateSuccess),
    ToolImmediateFailure(ToolImmediateFailure),
    ToolAsyncSpawned {
        run_id: String,
        tool_call_id: String,
        tool_name: String,
        job_id: String,
    },
    ToolAsyncCompleted {
        job_id: String,
    },
    ToolAsyncFailed {
        job_id: String,
        error: String,
    },
}

pub enum RuntimeCommand {
    CallLlm {
        run_id: String,
        assistant_message_id: String,
        messages: Vec<Message>,
        tools: LlmToolSet,
        cancel_token: CancellationToken,
    },
    CallTool {
        context: ToolExecutionContext,
        input: Value,
        tool: Option<LlmTool>,
        is_async: bool,
    },
}

#[derive(Debug, Clone)]
pub struct ToolImmediateSuccess {
    pub run_id: String,
    pub tool_call_id: String,
}

#[derive(Debug, Clone)]
pub struct ToolImmediateFailure {
    pub run_id: String,
    pub tool_call_id: String,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct ToolExecutionContext {
    pub session_id: String,
    pub run_id: String,
    pub message_id: String,
    pub part_id: String,
    pub tool_call_id: String,
    pub tool_name: String,
    pub input: Map<String, Value>,
    pub started_at: u64,
}
