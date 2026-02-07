use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartBase {
    pub id: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "messageId")]
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
    Tool(ToolPart),
    Source(SourcePart),
    File(FilePart),
}

impl MessagePart {
    pub fn base(&self) -> &PartBase {
        match self {
            MessagePart::Text(part) => &part.base,
            MessagePart::Reasoning(part) => &part.base,
            MessagePart::Tool(part) => &part.base,
            MessagePart::Source(part) => &part.base,
            MessagePart::File(part) => &part.base,
        }
    }

    pub fn base_mut(&mut self) -> &mut PartBase {
        match self {
            MessagePart::Text(part) => &mut part.base,
            MessagePart::Reasoning(part) => &mut part.base,
            MessagePart::Tool(part) => &mut part.base,
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
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ToolState {
    Pending(ToolStatePending),
    Running(ToolStateRunning),
    Completed(ToolStateCompleted),
    Error(ToolStateError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolStatePending {
    pub input: Map<String, Value>,
    pub raw: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolStateRunning {
    pub input: Map<String, Value>,
    pub title: Option<String>,
    pub metadata: Option<Map<String, Value>>,
    pub time: ToolStateTimeStart,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolStateCompleted {
    pub input: Map<String, Value>,
    pub output: String,
    pub title: String,
    pub metadata: Map<String, Value>,
    pub time: ToolStateTimeCompleted,
    pub attachments: Option<Vec<FilePart>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolStateError {
    pub input: Map<String, Value>,
    pub error: String,
    pub metadata: Option<Map<String, Value>>,
    pub time: ToolStateTimeRange,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolStateTimeStart {
    pub start: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolStateTimeRange {
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolStateTimeCompleted {
    pub start: u64,
    pub end: u64,
    pub compacted: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolPart {
    #[serde(flatten)]
    pub base: PartBase,
    #[serde(rename = "callId")]
    pub call_id: String,
    pub tool: String,
    pub state: ToolState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourcePart {
    #[serde(flatten)]
    pub base: PartBase,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sourceType")]
    pub source_type: String,
    pub url: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "mediaType")]
    pub media_type: Option<String>,
    pub filename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FilePart {
    #[serde(flatten)]
    pub base: PartBase,
    pub base64: String,
    #[serde(rename = "mediaType")]
    pub media_type: String,
    pub name: Option<String>,
}
