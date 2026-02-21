use crate::message::parts::{FilePart, MessagePart, ToolPart, ToolState};
use crate::message::Message;
use llm_kit_provider_utils::message::{
    AssistantContentPart, AssistantMessage, DataContent, FilePart as LlmFilePart,
    Message as LlmMessage, ReasoningPart as LlmReasoningPart, TextPart as LlmTextPart,
    ToolCallPart as LlmToolCallPart, ToolContentPart, ToolMessage, ToolResultOutput,
    ToolResultPart as LlmToolResultPart, UserContentPart, UserMessage,
};
use serde_json::Value;

pub fn messages_to_prompt(messages: &[Message]) -> Vec<LlmMessage> {
    messages.iter().flat_map(message_to_prompt).collect()
}

fn message_to_prompt(message: &Message) -> Vec<LlmMessage> {
    match message.info.role.as_str() {
        "user" => user_message_to_prompt(message),
        "assistant" => assistant_message_to_prompt(message),
        "tool" => tool_message_to_prompt(message),
        _ => Vec::new(),
    }
}

fn user_message_to_prompt(message: &Message) -> Vec<LlmMessage> {
    let parts: Vec<UserContentPart> = message
        .parts
        .iter()
        .filter_map(|part| match part {
            MessagePart::Text(text) => Some(UserContentPart::Text(LlmTextPart::new(&text.text))),
            MessagePart::File(file) => Some(UserContentPart::File(map_file_part(file))),
            _ => None,
        })
        .collect();
    if parts.is_empty() {
        return Vec::new();
    }
    vec![LlmMessage::User(UserMessage::with_parts(parts))]
}

fn assistant_message_to_prompt(message: &Message) -> Vec<LlmMessage> {
    let mut assistant_parts = Vec::new();
    let mut tool_parts = Vec::new();
    for part in &message.parts {
        match part {
            MessagePart::Text(text) => {
                assistant_parts.push(AssistantContentPart::Text(LlmTextPart::new(&text.text)));
            }
            MessagePart::Reasoning(reasoning) => {
                assistant_parts.push(AssistantContentPart::Reasoning(LlmReasoningPart::new(
                    &reasoning.text,
                )));
            }
            MessagePart::Tool(tool) => {
                if let Some(assistant_part) = map_tool_to_assistant_content(tool) {
                    assistant_parts.push(assistant_part);
                }
                if let Some(tool_part) = map_tool_to_tool_content(tool) {
                    tool_parts.push(tool_part);
                }
            }
            MessagePart::Extension(_) => {}
            MessagePart::File(file) => {
                assistant_parts.push(AssistantContentPart::File(map_file_part(file)));
            }
        }
    }
    let mut messages = Vec::new();
    if !assistant_parts.is_empty() {
        messages.push(LlmMessage::Assistant(AssistantMessage::with_parts(
            assistant_parts,
        )));
    }
    if !tool_parts.is_empty() {
        messages.push(LlmMessage::Tool(ToolMessage::new(tool_parts)));
    }
    messages
}

fn tool_message_to_prompt(message: &Message) -> Vec<LlmMessage> {
    let parts: Vec<ToolContentPart> = message
        .parts
        .iter()
        .filter_map(|part| match part {
            MessagePart::Tool(tool) => map_tool_to_tool_content(tool),
            _ => None,
        })
        .collect();
    if parts.is_empty() {
        return Vec::new();
    }
    vec![LlmMessage::Tool(ToolMessage::new(parts))]
}

fn map_tool_to_assistant_content(tool: &ToolPart) -> Option<AssistantContentPart> {
    let input = match &tool.state {
        ToolState::Pending(state) => state.input.clone(),
        ToolState::Running(state) => state.input.clone(),
        ToolState::Completed(state) => state.input.clone(),
        ToolState::Error(state) => state.input.clone(),
    };
    Some(AssistantContentPart::ToolCall(LlmToolCallPart::new(
        tool.call_id.clone(),
        tool.tool.clone(),
        Value::Object(input),
    )))
}

fn map_tool_to_tool_content(tool: &ToolPart) -> Option<ToolContentPart> {
    let output = match &tool.state {
        ToolState::Completed(state) => ToolResultOutput::text(&state.output),
        ToolState::Error(state) => ToolResultOutput::error_text(&state.error),
        ToolState::Pending(_) | ToolState::Running(_) => return None,
    };
    Some(ToolContentPart::ToolResult(LlmToolResultPart::new(
        tool.call_id.clone(),
        tool.tool.clone(),
        output,
    )))
}

fn map_file_part(file: &FilePart) -> LlmFilePart {
    let part = LlmFilePart::from_data(
        DataContent::base64(file.base64.clone()),
        file.media_type.clone(),
    );
    match &file.name {
        Some(name) => part.with_filename(name.clone()),
        None => part,
    }
}
