//! Platform abstraction traits (Core-11).
//!
//! Defines traits for OS-level capabilities so the core crate never depends
//! on platform-specific APIs directly.

use serde_json::Value;

/// Abstraction over clipboard access.
///
/// Implementations provide clipboard history to built-in tools without
/// requiring OS-specific imports in the core crate.
pub trait ClipboardProvider: Send + Sync {
    /// Returns all clipboard history entries (oldest first).
    fn get_history(&self) -> Vec<Value>;

    /// Returns the most recent clipboard entry, if any.
    fn get_latest(&self) -> Option<Value>;
}

/// A no-op provider that always returns empty results.
///
/// Use when no real clipboard is available (headless, CI, etc.).
pub struct NullClipboardProvider;

impl ClipboardProvider for NullClipboardProvider {
    fn get_history(&self) -> Vec<Value> {
        vec![]
    }

    fn get_latest(&self) -> Option<Value> {
        None
    }
}

/// A test-only provider pre-loaded with entries.
pub struct MockClipboardProvider {
    entries: Vec<Value>,
}

impl MockClipboardProvider {
    pub fn new(entries: Vec<Value>) -> Self {
        Self { entries }
    }
}

impl ClipboardProvider for MockClipboardProvider {
    fn get_history(&self) -> Vec<Value> {
        self.entries.clone()
    }

    fn get_latest(&self) -> Option<Value> {
        self.entries.last().cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn null_provider_returns_empty() {
        let provider = NullClipboardProvider;
        assert!(provider.get_history().is_empty());
        assert!(provider.get_latest().is_none());
    }

    #[test]
    fn mock_provider_returns_entries() {
        let provider = MockClipboardProvider::new(vec![
            json!({"text": "first"}),
            json!({"text": "second"}),
        ]);
        assert_eq!(provider.get_history().len(), 2);
        assert_eq!(provider.get_latest().unwrap()["text"], "second");
    }
}
