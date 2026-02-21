use std::pin::Pin;

use futures_util::Stream;
use serde_json::Value;

/// Provider-agnostic stream event enum.
#[derive(Debug, Clone)]
pub enum LlmStreamEvent {
    TextStart {
        id: String,
    },
    TextDelta {
        id: String,
        text: String,
    },
    TextEnd {
        id: String,
    },
    ReasoningStart {
        id: String,
    },
    ReasoningDelta {
        id: String,
        text: String,
    },
    ReasoningEnd {
        id: String,
    },
    ToolCall {
        tool_call_id: String,
        tool_name: String,
        input: Value,
    },
    ToolResult {
        tool_call_id: String,
        tool_name: String,
        input: Value,
        output: Value,
    },
    ToolError {
        tool_call_id: String,
        tool_name: String,
        input: Value,
        error: Value,
    },
    File {
        base64: String,
        media_type: String,
        name: Option<String>,
    },
    Error {
        error: Value,
    },
    Start,
    Finish,
}

pub type LlmStream = Pin<Box<dyn Stream<Item = LlmStreamEvent> + Send>>;
