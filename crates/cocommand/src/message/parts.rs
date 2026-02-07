use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartBase {
    pub id: String,
    #[serde(rename = "sessionID", alias = "sessionId", alias = "session_id")]
    pub session_id: String,
    #[serde(rename = "messageID", alias = "messageId", alias = "message_id")]
    pub message_id: String,
}

impl PartBase {
    pub fn new(session_id: impl Into<String>, message_id: impl Into<String>) -> Self {
        Self {
            id: Uuid::now_v7().to_string(),
            session_id: session_id.into(),
            message_id: message_id.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum MessagePart {
    Text(TextPart),
    Reasoning(ReasoningPart),
    ToolCall(ToolCallPart),
    ToolResult(ToolResultPart),
    ToolError(ToolErrorPart),
    Source(SourcePart),
    File(FilePart),
}

impl MessagePart {
    pub fn base(&self) -> &PartBase {
        match self {
            MessagePart::Text(part) => &part.base,
            MessagePart::Reasoning(part) => &part.base,
            MessagePart::ToolCall(part) => &part.base,
            MessagePart::ToolResult(part) => &part.base,
            MessagePart::ToolError(part) => &part.base,
            MessagePart::Source(part) => &part.base,
            MessagePart::File(part) => &part.base,
        }
    }

    pub fn base_mut(&mut self) -> &mut PartBase {
        match self {
            MessagePart::Text(part) => &mut part.base,
            MessagePart::Reasoning(part) => &mut part.base,
            MessagePart::ToolCall(part) => &mut part.base,
            MessagePart::ToolResult(part) => &mut part.base,
            MessagePart::ToolError(part) => &mut part.base,
            MessagePart::Source(part) => &mut part.base,
            MessagePart::File(part) => &mut part.base,
        }
    }

    pub fn with_base(mut self, base: PartBase) -> Self {
        *self.base_mut() = base;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReasoningPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCallPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub call_id: String,
    pub tool_name: String,
    pub input: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolResultPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub call_id: String,
    pub tool_name: String,
    pub output: Value,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolErrorPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub call_id: String,
    pub tool_name: String,
    pub error: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourcePart {
    #[serde(flatten)]
    pub base: PartBase,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    pub source_type: String,
    pub url: Option<String>,
    pub title: Option<String>,
    pub media_type: Option<String>,
    pub filename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FilePart {
    #[serde(flatten)]
    pub base: PartBase,
    pub base64: String,
    pub media_type: String,
    pub name: Option<String>,
}
