//! Session management for the cocommand agent.
//!
//! This module provides session state management with:
//! - Message types for conversation tracking
//! - Tool call and result types
//! - Session phases (Control/Execution)
//! - Session state and lifecycle management
//!
//! # Submodules
//!
//! - `message`: Message and message part types
//! - `tool`: Tool call and result types
//! - `phase`: Session phase definitions
//! - `state`: Session state management

pub mod message;
pub mod phase;
pub mod state;
pub mod tool;

// Re-export commonly used items for convenience
pub use message::{Message, MessagePart, MessageRole};
pub use phase::SessionPhase;
pub use state::Session;
pub use tool::{ToolCall, ToolResult};

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
