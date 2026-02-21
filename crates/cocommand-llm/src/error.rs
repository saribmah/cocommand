use std::fmt;

#[derive(Debug, Clone)]
pub enum LlmError {
    InvalidInput(String),
    Internal(String),
    MissingApiKey,
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            LlmError::Internal(msg) => write!(f, "internal error: {msg}"),
            LlmError::MissingApiKey => write!(f, "missing LLM API key"),
        }
    }
}

impl std::error::Error for LlmError {}
