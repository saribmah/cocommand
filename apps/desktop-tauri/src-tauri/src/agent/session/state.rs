//! Session state management.
//!
//! This module contains the Session struct that tracks the conversation
//! and execution context for a single agent interaction.

use super::message::{Message, MessageRole};
use super::phase::SessionPhase;

/// Session state that tracks the conversation and execution context.
#[derive(Clone, Debug)]
pub struct Session {
    pub id: String,
    pub messages: Vec<Message>,
    pub phase: SessionPhase,
    pub turn_count: u32,
    pub max_turns: u32,
}

impl Session {
    /// Create a new session with the given ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            messages: Vec::new(),
            phase: SessionPhase::Control,
            turn_count: 0,
            max_turns: 10, // Default max turns to prevent infinite loops
        }
    }

    /// Set the maximum number of turns for this session.
    pub fn with_max_turns(mut self, max_turns: u32) -> Self {
        self.max_turns = max_turns;
        self
    }

    /// Add a message to the session.
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Set the current phase of the session.
    pub fn set_phase(&mut self, phase: SessionPhase) {
        self.phase = phase;
    }

    /// Increment the turn counter.
    pub fn increment_turn(&mut self) {
        self.turn_count += 1;
    }

    /// Check if the session can continue (hasn't exceeded max turns).
    pub fn can_continue(&self) -> bool {
        self.turn_count < self.max_turns
    }

    /// Get the last assistant message text, if any.
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
