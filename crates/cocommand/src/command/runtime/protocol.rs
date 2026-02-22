use serde_json::Value;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::command::runtime::types::EnqueueMessageAck;
use crate::command::session_message::SessionCommandInputPart;
use crate::error::CoreResult;
use crate::llm::{LlmStreamEvent, LlmTool, LlmToolSet};
use crate::message::Message;

pub enum SessionEvent {
    UserMessage {
        parts: Vec<SessionCommandInputPart>,
        reply: oneshot::Sender<CoreResult<EnqueueMessageAck>>,
    },
    LlmStreamPart {
        run_id: String,
        part: LlmStreamEvent,
    },
    LlmFinished {
        run_id: String,
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
        output: Value,
    },
    ToolAsyncFailed {
        job_id: String,
        error: Value,
    },
}

pub enum RuntimeCommand {
    CallLlm {
        run_id: String,
        messages: Vec<Message>,
        tools: LlmToolSet,
        cancel_token: CancellationToken,
    },
    CallTool {
        run_id: String,
        tool_call_id: String,
        tool_name: String,
        input: Value,
        tool: Option<LlmTool>,
        is_async: bool,
    },
}

#[derive(Debug, Clone)]
pub struct ToolImmediateSuccess {
    pub run_id: String,
    pub tool_call_id: String,
    pub output: Value,
}

#[derive(Debug, Clone)]
pub struct ToolImmediateFailure {
    pub run_id: String,
    pub tool_call_id: String,
    pub error: Value,
}
