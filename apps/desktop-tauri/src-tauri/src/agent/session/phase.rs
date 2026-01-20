//! Session phase definitions.
//!
//! This module defines the execution phases for the agent loop.

/// Represents which phase of the agent loop we're in.
#[derive(Clone, Debug, PartialEq)]
pub enum SessionPhase {
    /// Control plane: only window.* tools available.
    Control,
    /// Execution plane: window.* tools + app tools for open apps.
    Execution,
}

impl SessionPhase {
    /// Returns the string representation of the phase.
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionPhase::Control => "control",
            SessionPhase::Execution => "execution",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_as_str() {
        assert_eq!(SessionPhase::Control.as_str(), "control");
        assert_eq!(SessionPhase::Execution.as_str(), "execution");
    }

    #[test]
    fn test_phase_equality() {
        assert_eq!(SessionPhase::Control, SessionPhase::Control);
        assert_ne!(SessionPhase::Control, SessionPhase::Execution);
    }
}
