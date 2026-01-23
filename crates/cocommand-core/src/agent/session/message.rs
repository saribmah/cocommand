//! Message types for the session.
//!
//! This module contains the message structure used in agent conversations,
//! including text, tool calls, and tool results.

use serde::{Deserialize, Serialize};

use super::tool::{ToolCall, ToolResult};

/// A message part that can be text, tool call, or tool result.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessagePart {
    #[serde(rename = "text")]
    Text { content: String },
    #[serde(rename = "tool_call")]
    ToolCall(ToolCall),
    #[serde(rename = "tool_result")]
    ToolResult(ToolResult),
}

/// Role of a message in the conversation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A message in the session, which may contain multiple parts.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub parts: Vec<MessagePart>,
}

impl Message {
    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            parts: vec![MessagePart::Text {
                content: content.into(),
            }],
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            parts: vec![MessagePart::Text {
                content: content.into(),
            }],
        }
    }

    /// Create an assistant message with text content.
    pub fn assistant_text(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            parts: vec![MessagePart::Text {
                content: content.into(),
            }],
        }
    }

    /// Create an assistant message with tool calls.
    pub fn assistant_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: MessageRole::Assistant,
            parts: tool_calls.into_iter().map(MessagePart::ToolCall).collect(),
        }
    }

    /// Create a message containing tool results.
    pub fn tool_results(results: Vec<ToolResult>) -> Self {
        Self {
            role: MessageRole::Tool,
            parts: results.into_iter().map(MessagePart::ToolResult).collect(),
        }
    }

    /// Extract text content from the message, if any.
    pub fn text_content(&self) -> Option<String> {
        self.parts
            .iter()
            .filter_map(|part| match part {
                MessagePart::Text { content } => Some(content.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
            .into()
    }

    /// Extract tool calls from the message, if any.
    pub fn tool_calls(&self) -> Vec<&ToolCall> {
        self.parts
            .iter()
            .filter_map(|part| match part {
                MessagePart::ToolCall(tc) => Some(tc),
                _ => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let user_msg = Message::user("Hello");
        assert_eq!(user_msg.role, MessageRole::User);
        assert_eq!(user_msg.text_content(), Some("Hello".to_string()));

        let system_msg = Message::system("You are an assistant");
        assert_eq!(system_msg.role, MessageRole::System);
    }
}
