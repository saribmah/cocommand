//! Namespaced key-value store trait and in-memory implementation.

use std::collections::HashMap;

/// Namespaced key-value store for settings and metadata.
///
/// All operations are scoped by namespace to prevent collisions between
/// different domains (e.g. settings, extension state, preferences).
pub trait KvStore: Send + Sync {
    fn get(&self, namespace: &str, key: &str) -> Option<serde_json::Value>;
    fn set(&mut self, namespace: &str, key: &str, value: serde_json::Value);
    fn delete(&mut self, namespace: &str, key: &str) -> bool;
    /// Return all keys within the given namespace.
    fn keys(&self, namespace: &str) -> Vec<String>;
}

// --- Memory Implementation ---

#[derive(Debug, Default)]
pub(crate) struct MemoryKvStore {
    data: HashMap<String, HashMap<String, serde_json::Value>>,
}

impl KvStore for MemoryKvStore {
    fn get(&self, namespace: &str, key: &str) -> Option<serde_json::Value> {
        self.data.get(namespace).and_then(|ns| ns.get(key).cloned())
    }

    fn set(&mut self, namespace: &str, key: &str, value: serde_json::Value) {
        self.data
            .entry(namespace.to_string())
            .or_default()
            .insert(key.to_string(), value);
    }

    fn delete(&mut self, namespace: &str, key: &str) -> bool {
        if let Some(ns) = self.data.get_mut(namespace) {
            ns.remove(key).is_some()
        } else {
            false
        }
    }

    fn keys(&self, namespace: &str) -> Vec<String> {
        self.data
            .get(namespace)
            .map(|ns| {
                let mut keys: Vec<String> = ns.keys().cloned().collect();
                keys.sort();
                keys
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn get_missing_returns_none() {
        let kv = MemoryKvStore::default();
        assert!(kv.get("settings", "nonexistent").is_none());
    }

    #[test]
    fn set_and_get() {
        let mut kv = MemoryKvStore::default();
        kv.set("settings", "theme", json!("dark"));
        assert_eq!(kv.get("settings", "theme"), Some(json!("dark")));
    }

    #[test]
    fn namespaces_are_isolated() {
        let mut kv = MemoryKvStore::default();
        kv.set("settings", "theme", json!("dark"));
        kv.set("extensions", "theme", json!("light"));

        assert_eq!(kv.get("settings", "theme"), Some(json!("dark")));
        assert_eq!(kv.get("extensions", "theme"), Some(json!("light")));
    }

    #[test]
    fn delete_existing_key() {
        let mut kv = MemoryKvStore::default();
        kv.set("settings", "key", json!(1));
        assert!(kv.delete("settings", "key"));
        assert!(!kv.delete("settings", "key"));
        assert!(kv.get("settings", "key").is_none());
    }

    #[test]
    fn delete_wrong_namespace_returns_false() {
        let mut kv = MemoryKvStore::default();
        kv.set("settings", "key", json!(1));
        assert!(!kv.delete("other", "key"));
        assert_eq!(kv.get("settings", "key"), Some(json!(1)));
    }

    #[test]
    fn keys_returns_sorted_namespace_keys() {
        let mut kv = MemoryKvStore::default();
        kv.set("settings", "b", json!(2));
        kv.set("settings", "a", json!(1));
        kv.set("extensions", "c", json!(3));

        // Returned in sorted order regardless of insertion order.
        assert_eq!(kv.keys("settings"), vec!["a", "b"]);
        assert_eq!(kv.keys("extensions"), vec!["c"]);
        assert!(kv.keys("unknown").is_empty());
    }
}
