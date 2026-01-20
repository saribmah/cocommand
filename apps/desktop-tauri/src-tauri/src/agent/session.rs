use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents a tool call made by the assistant
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// Represents the result of a tool execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: Value,
    pub is_error: bool,
}

/// A message part that can be text, tool call, or tool result
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

/// Role of a message in the conversation
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A message in the session, which may contain multiple parts
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub parts: Vec<MessagePart>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            parts: vec![MessagePart::Text {
                content: content.into(),
            }],
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            parts: vec![MessagePart::Text {
                content: content.into(),
            }],
        }
    }

    pub fn assistant_text(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            parts: vec![MessagePart::Text {
                content: content.into(),
            }],
        }
    }

    pub fn assistant_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: MessageRole::Assistant,
            parts: tool_calls.into_iter().map(MessagePart::ToolCall).collect(),
        }
    }

    pub fn tool_results(results: Vec<ToolResult>) -> Self {
        Self {
            role: MessageRole::Tool,
            parts: results.into_iter().map(MessagePart::ToolResult).collect(),
        }
    }

    /// Extract text content from the message, if any
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

    /// Extract tool calls from the message, if any
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

/// Represents which phase of the agent loop we're in
#[derive(Clone, Debug, PartialEq)]
pub enum SessionPhase {
    /// Control plane: only window.* tools available
    Control,
    /// Execution plane: window.* tools + app tools for open apps
    Execution,
}

/// Session state that tracks the conversation and execution context
#[derive(Clone, Debug)]
pub struct Session {
    pub id: String,
    pub messages: Vec<Message>,
    pub phase: SessionPhase,
    pub turn_count: u32,
    pub max_turns: u32,
}

impl Session {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            messages: Vec::new(),
            phase: SessionPhase::Control,
            turn_count: 0,
            max_turns: 10, // Default max turns to prevent infinite loops
        }
    }

    pub fn with_max_turns(mut self, max_turns: u32) -> Self {
        self.max_turns = max_turns;
        self
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn set_phase(&mut self, phase: SessionPhase) {
        self.phase = phase;
    }

    pub fn increment_turn(&mut self) {
        self.turn_count += 1;
    }

    pub fn can_continue(&self) -> bool {
        self.turn_count < self.max_turns
    }

    /// Get the last assistant message text, if any
    pub fn last_assistant_text(&self) -> Option<String> {
        self.messages
            .iter()
            .rev()
            .find(|m| m.role == MessageRole::Assistant)
            .and_then(|m| {
                let text = m.text_content();
                if text.as_ref().map(|t| t.is_empty()).unwrap_or(true) {
                    None
                } else {
                    text
                }
            })
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

    #[test]
    fn test_session_phases() {
        let mut session = Session::new("test-session");
        assert_eq!(session.phase, SessionPhase::Control);

        session.set_phase(SessionPhase::Execution);
        assert_eq!(session.phase, SessionPhase::Execution);
    }

    #[test]
    fn test_session_turns() {
        let mut session = Session::new("test-session").with_max_turns(3);
        assert!(session.can_continue());

        session.increment_turn();
        session.increment_turn();
        assert!(session.can_continue());

        session.increment_turn();
        assert!(!session.can_continue());
    }
}
