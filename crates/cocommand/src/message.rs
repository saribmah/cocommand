pub mod message;

// Re-export types from cocommand-llm
pub use cocommand_llm::message::info;
pub use cocommand_llm::message::parts;

pub use cocommand_llm::message::info::{AssistantMessageInfo, MessageInfo, UserMessageInfo};
pub use cocommand_llm::message::Message;
pub use cocommand_llm::message::parts::{
    ExtensionPart, FilePart, FilePartFileSource, FilePartSource, FilePartSourceText,
    FilePartSymbolSource, MessagePart, PartBase, ReasoningPart, TextPart, ToolPart, ToolState,
    ToolStateCompleted, ToolStateError, ToolStatePending, ToolStateRunning, ToolStateTimeCompleted,
    ToolStateTimeRange, ToolStateTimeStart,
};
