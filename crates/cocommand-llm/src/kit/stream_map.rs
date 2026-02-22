use futures_util::StreamExt;
use llm_kit_core::stream_text::TextStreamPart;

use crate::stream::{LlmStream, LlmStreamEvent};

/// Convert a llm-kit `TextStreamPart` stream into our `LlmStream`.
pub fn map_kit_stream<S>(stream: S) -> LlmStream
where
    S: futures_util::Stream<Item = TextStreamPart> + Send + 'static,
{
    Box::pin(stream.filter_map(|part| async move { map_part(part) }))
}

fn map_part(part: TextStreamPart) -> Option<LlmStreamEvent> {
    match part {
        TextStreamPart::TextStart { id, .. } => Some(LlmStreamEvent::TextStart { id }),
        TextStreamPart::TextDelta { id, text, .. } => Some(LlmStreamEvent::TextDelta { id, text }),
        TextStreamPart::TextEnd { id, .. } => Some(LlmStreamEvent::TextEnd { id }),
        TextStreamPart::ReasoningStart { id, .. } => Some(LlmStreamEvent::ReasoningStart { id }),
        TextStreamPart::ReasoningDelta { id, text, .. } => {
            Some(LlmStreamEvent::ReasoningDelta { id, text })
        }
        TextStreamPart::ReasoningEnd { id, .. } => Some(LlmStreamEvent::ReasoningEnd { id }),
        TextStreamPart::ToolCall { tool_call } => Some(LlmStreamEvent::ToolCall {
            tool_call_id: tool_call.tool_call_id,
            tool_name: tool_call.tool_name,
            input: tool_call.input,
        }),
        TextStreamPart::ToolResult { tool_result } => Some(LlmStreamEvent::ToolResult {
            tool_call_id: tool_result.tool_call_id,
            tool_name: tool_result.tool_name,
            input: tool_result.input,
            output: tool_result.output,
        }),
        TextStreamPart::ToolError { tool_error } => Some(LlmStreamEvent::ToolError {
            tool_call_id: tool_error.tool_call_id,
            tool_name: tool_error.tool_name,
            input: tool_error.input,
            error: tool_error.error,
        }),
        TextStreamPart::File { file } => Some(LlmStreamEvent::File {
            base64: file.base64,
            media_type: file.media_type,
            name: file.name,
        }),
        TextStreamPart::Error { error } => Some(LlmStreamEvent::Error { error }),
        TextStreamPart::Start => Some(LlmStreamEvent::Start),
        TextStreamPart::Finish { .. } => Some(LlmStreamEvent::Finish),
        // Drop metadata-only events that cocommand doesn't use
        TextStreamPart::ToolInputStart { .. }
        | TextStreamPart::ToolInputDelta { .. }
        | TextStreamPart::ToolInputEnd { .. }
        | TextStreamPart::ToolOutputDenied { .. }
        | TextStreamPart::ToolApprovalRequest { .. }
        | TextStreamPart::StartStep { .. }
        | TextStreamPart::FinishStep { .. }
        | TextStreamPart::Abort
        | TextStreamPart::Source { .. }
        | TextStreamPart::Raw { .. } => None,
    }
}
