use serde::{Deserialize, Serialize};
use std::fmt;

/// Unified error type for the cocommand crate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoreError {
    /// Functionality not yet implemented.
    NotImplemented,
    /// Invalid input provided by the caller.
    InvalidInput(String),
    /// Internal error.
    Internal(String),
    /// Post-mutation invariant was violated.
    InvariantViolation(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::NotImplemented => write!(f, "not implemented"),
            CoreError::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            CoreError::Internal(msg) => write!(f, "internal error: {msg}"),
            CoreError::InvariantViolation(msg) => write!(f, "invariant violation: {msg}"),
        }
    }
}

impl std::error::Error for CoreError {}

impl From<filesystem::FilesystemError> for CoreError {
    fn from(err: filesystem::FilesystemError) -> Self {
        match err {
            filesystem::FilesystemError::InvalidInput(msg) => CoreError::InvalidInput(msg),
            other => CoreError::Internal(other.to_string()),
        }
    }
}

/// Result type alias using [`CoreError`].
pub type CoreResult<T> = Result<T, CoreError>;
