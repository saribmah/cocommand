pub mod info;
pub mod parts;

pub use info::{AssistantMessageInfo, MessageInfo, UserMessageInfo};
pub use parts::{
    ExtensionPart, FilePart, FilePartFileSource, FilePartSource, FilePartSourceText,
    FilePartSymbolSource, MessagePart, PartBase, ReasoningPart, TextPart, ToolPart, ToolState,
    ToolStateCompleted, ToolStateError, ToolStatePending, ToolStateRunning, ToolStateTimeCompleted,
    ToolStateTimeRange, ToolStateTimeStart,
};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

fn now_rfc3339() -> String {
    let now = std::time::SystemTime::now();
    let datetime: chrono::DateTime<chrono::Utc> = now.into();
    datetime.to_rfc3339()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct Message {
    pub info: MessageInfo,
    pub parts: Vec<MessagePart>,
}

impl Message {
    pub fn from_text(session_id: &str, role: &str, text: &str) -> Message {
        let timestamp = now_rfc3339();
        let info = MessageInfo {
            id: Uuid::now_v7().to_string(),
            session_id: session_id.to_string(),
            role: role.to_string(),
            created_at: timestamp.clone(),
            completed_at: None,
        };
        Message {
            info: info.clone(),
            parts: vec![MessagePart::Text(TextPart {
                base: PartBase::new(session_id, info.id.as_str()),
                text: text.to_string(),
            })],
        }
    }

    pub fn from_parts(session_id: &str, role: &str, parts: Vec<MessagePart>) -> Message {
        let timestamp = now_rfc3339();
        let info = MessageInfo {
            id: Uuid::now_v7().to_string(),
            session_id: session_id.to_string(),
            role: role.to_string(),
            created_at: timestamp.clone(),
            completed_at: None,
        };
        let parts = parts
            .into_iter()
            .map(|part| part.with_base(PartBase::new(session_id, info.id.as_str())))
            .collect();
        Message { info, parts }
    }
}
