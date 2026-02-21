use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum MessagePart {
    Text(TextPart),
    Reasoning(ReasoningPart),
    Tool(ToolPart),
    Extension(ExtensionPart),
    File(FilePart),
}

impl MessagePart {
    pub fn base(&self) -> &PartBase {
        match self {
            MessagePart::Text(part) => &part.base,
            MessagePart::Reasoning(part) => &part.base,
            MessagePart::Tool(part) => &part.base,
            MessagePart::Extension(part) => &part.base,
            MessagePart::File(part) => &part.base,
        }
    }

    pub fn base_mut(&mut self) -> &mut PartBase {
        match self {
            MessagePart::Text(part) => &mut part.base,
            MessagePart::Reasoning(part) => &mut part.base,
            MessagePart::Tool(part) => &mut part.base,
            MessagePart::Extension(part) => &mut part.base,
            MessagePart::File(part) => &mut part.base,
        }
    }

    pub fn with_base(mut self, base: PartBase) -> Self {
        *self.base_mut() = base;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct TextPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ReasoningPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ToolState {
    Pending(ToolStatePending),
    Running(ToolStateRunning),
    Completed(ToolStateCompleted),
    Error(ToolStateError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ToolStatePending {
    pub input: Map<String, Value>,
    pub raw: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ToolStateRunning {
    pub input: Map<String, Value>,
    pub title: Option<String>,
    pub metadata: Option<Map<String, Value>>,
    pub time: ToolStateTimeStart,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ToolStateCompleted {
    pub input: Map<String, Value>,
    pub output: String,
    pub title: String,
    pub metadata: Map<String, Value>,
    pub time: ToolStateTimeCompleted,
    pub attachments: Option<Vec<FilePart>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ToolStateError {
    pub input: Map<String, Value>,
    pub error: String,
    pub metadata: Option<Map<String, Value>>,
    pub time: ToolStateTimeRange,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ToolStateTimeStart {
    pub start: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ToolStateTimeRange {
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ToolStateTimeCompleted {
    pub start: u64,
    pub end: u64,
    pub compacted: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ExtensionPart {
    #[serde(flatten)]
    pub base: PartBase,
    #[serde(rename = "extensionId")]
    pub extension_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<FilePartSourceText>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct FilePartSourceText {
    pub value: String,
    pub start: i64,
    pub end: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FilePartSource {
    File(FilePartFileSource),
    Symbol(FilePartSymbolSource),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct FilePartFileSource {
    pub text: FilePartSourceText,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct FilePartSymbolSource {
    pub text: FilePartSourceText,
    pub path: String,
    pub name: String,
    pub kind: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct FilePart {
    #[serde(flatten)]
    pub base: PartBase,
    pub base64: String,
    #[serde(rename = "mediaType")]
    pub media_type: String,
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<FilePartSource>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_part_serializes_as_extension_type() {
        let part = MessagePart::Extension(ExtensionPart {
            base: PartBase {
                id: "part_1".to_string(),
                session_id: "session_1".to_string(),
                message_id: "message_1".to_string(),
            },
            extension_id: "filesystem".to_string(),
            name: "Filesystem".to_string(),
            kind: None,
            source: Some(FilePartSourceText {
                value: "@filesystem".to_string(),
                start: 0,
                end: 11,
            }),
        });

        let value = serde_json::to_value(part).expect("part should serialize");
        assert_eq!(value["type"], "extension");
        assert_eq!(value["extensionId"], "filesystem");
        assert_eq!(value["name"], "Filesystem");
        assert_eq!(value["source"]["value"], "@filesystem");
        assert_eq!(value["source"]["start"], 0);
        assert_eq!(value["source"]["end"], 11);
        assert!(value.get("kind").is_none());
    }

    #[test]
    fn file_part_source_file_serializes_with_text_range() {
        let source = FilePartSource::File(FilePartFileSource {
            text: FilePartSourceText {
                value: "match this".to_string(),
                start: 4,
                end: 14,
            },
            path: "/tmp/example.rs".to_string(),
        });

        let value = serde_json::to_value(source).expect("source should serialize");

        assert_eq!(value["type"], "file");
        assert_eq!(value["path"], "/tmp/example.rs");
        assert_eq!(value["text"]["value"], "match this");
        assert_eq!(value["text"]["start"], 4);
        assert_eq!(value["text"]["end"], 14);
    }

    #[test]
    fn file_part_source_symbol_deserializes() {
        let value = serde_json::json!({
            "type": "symbol",
            "path": "/tmp/example.rs",
            "name": "run",
            "kind": 12,
            "text": {
                "value": "fn run()",
                "start": 0,
                "end": 8
            }
        });

        let source: FilePartSource =
            serde_json::from_value(value).expect("source should deserialize");
        match source {
            FilePartSource::Symbol(symbol) => {
                assert_eq!(symbol.path, "/tmp/example.rs");
                assert_eq!(symbol.name, "run");
                assert_eq!(symbol.kind, 12);
                assert_eq!(symbol.text.value, "fn run()");
                assert_eq!(symbol.text.start, 0);
                assert_eq!(symbol.text.end, 8);
            }
            FilePartSource::File(_) => panic!("expected symbol source"),
        }
    }

    #[test]
    fn file_part_without_source_omits_source_field() {
        let part = FilePart {
            base: PartBase {
                id: "part_1".to_string(),
                session_id: "session_1".to_string(),
                message_id: "message_1".to_string(),
            },
            base64: "dGVzdA==".to_string(),
            media_type: "text/plain".to_string(),
            name: Some("note.txt".to_string()),
            source: None,
        };

        let value = serde_json::to_value(part).expect("file part should serialize");
        assert!(value.get("source").is_none());
    }
}
