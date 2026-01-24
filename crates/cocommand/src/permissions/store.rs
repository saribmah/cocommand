//! In-memory permission decision store (session-scoped).

use std::collections::HashMap;
use super::scopes::PermissionScope;

/// A permission decision for a (tool, scope) pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    /// The tool is allowed to execute without confirmation.
    Allow,
    /// The user should be asked before execution.
    Ask,
    /// The tool is denied execution.
    Deny,
}

/// Session-scoped store for permission decisions.
///
/// Decisions are keyed by `(tool_id, scope)` and live only for the
/// duration of the session (no persistence).
pub struct PermissionStore {
    decisions: HashMap<(String, PermissionScope), PermissionDecision>,
}

impl PermissionStore {
    /// Create a new empty permission store.
    pub fn new() -> Self {
        Self {
            decisions: HashMap::new(),
        }
    }

    /// Store a permission decision for a (tool_id, scope) pair.
    pub fn set_decision(&mut self, tool_id: String, scope: PermissionScope, decision: PermissionDecision) {
        self.decisions.insert((tool_id, scope), decision);
    }

    /// Retrieve the stored decision for a (tool_id, scope) pair.
    pub fn get_decision(&self, tool_id: &str, scope: &PermissionScope) -> Option<&PermissionDecision> {
        self.decisions.get(&(tool_id.to_string(), scope.clone()))
    }

    /// Remove a stored decision for a (tool_id, scope) pair.
    pub fn clear_decision(&mut self, tool_id: &str, scope: &PermissionScope) {
        self.decisions.remove(&(tool_id.to_string(), scope.clone()));
    }
}

impl Default for PermissionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_store_returns_none() {
        let store = PermissionStore::new();
        assert_eq!(store.get_decision("tool_a", &PermissionScope::Read), None);
    }

    #[test]
    fn set_and_get_decision() {
        let mut store = PermissionStore::new();
        store.set_decision("tool_a".to_string(), PermissionScope::Write, PermissionDecision::Allow);

        assert_eq!(
            store.get_decision("tool_a", &PermissionScope::Write),
            Some(&PermissionDecision::Allow)
        );
    }

    #[test]
    fn different_scopes_are_independent() {
        let mut store = PermissionStore::new();
        store.set_decision("tool_a".to_string(), PermissionScope::Read, PermissionDecision::Allow);
        store.set_decision("tool_a".to_string(), PermissionScope::Write, PermissionDecision::Deny);

        assert_eq!(
            store.get_decision("tool_a", &PermissionScope::Read),
            Some(&PermissionDecision::Allow)
        );
        assert_eq!(
            store.get_decision("tool_a", &PermissionScope::Write),
            Some(&PermissionDecision::Deny)
        );
    }

    #[test]
    fn clear_decision_removes_entry() {
        let mut store = PermissionStore::new();
        store.set_decision("tool_a".to_string(), PermissionScope::Read, PermissionDecision::Allow);
        store.clear_decision("tool_a", &PermissionScope::Read);

        assert_eq!(store.get_decision("tool_a", &PermissionScope::Read), None);
    }

    #[test]
    fn overwrite_decision() {
        let mut store = PermissionStore::new();
        store.set_decision("tool_a".to_string(), PermissionScope::Read, PermissionDecision::Allow);
        store.set_decision("tool_a".to_string(), PermissionScope::Read, PermissionDecision::Deny);

        assert_eq!(
            store.get_decision("tool_a", &PermissionScope::Read),
            Some(&PermissionDecision::Deny)
        );
    }
}
