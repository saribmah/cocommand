//! Generic key-value store trait and in-memory implementation.

use std::collections::HashMap;

/// Generic key-value store for settings and metadata.
pub trait KvStore: Send + Sync {
    fn get(&self, key: &str) -> Option<serde_json::Value>;
    fn set(&mut self, key: &str, value: serde_json::Value);
    fn delete(&mut self, key: &str) -> bool;
    fn keys(&self) -> Vec<String>;
}

// --- Memory Implementation ---

#[derive(Debug, Default)]
pub(crate) struct MemoryKvStore {
    data: HashMap<String, serde_json::Value>,
}

impl KvStore for MemoryKvStore {
    fn get(&self, key: &str) -> Option<serde_json::Value> {
        self.data.get(key).cloned()
    }

    fn set(&mut self, key: &str, value: serde_json::Value) {
        self.data.insert(key.to_string(), value);
    }

    fn delete(&mut self, key: &str) -> bool {
        self.data.remove(key).is_some()
    }

    fn keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn get_missing_returns_none() {
        let kv = MemoryKvStore::default();
        assert!(kv.get("nonexistent").is_none());
    }

    #[test]
    fn set_and_get() {
        let mut kv = MemoryKvStore::default();
        kv.set("theme", json!("dark"));
        assert_eq!(kv.get("theme"), Some(json!("dark")));
    }

    #[test]
    fn delete_existing_key() {
        let mut kv = MemoryKvStore::default();
        kv.set("key", json!(1));
        assert!(kv.delete("key"));
        assert!(!kv.delete("key"));
        assert!(kv.get("key").is_none());
    }

    #[test]
    fn keys_returns_all() {
        let mut kv = MemoryKvStore::default();
        kv.set("a", json!(1));
        kv.set("b", json!(2));

        let mut keys = kv.keys();
        keys.sort();
        assert_eq!(keys, vec!["a", "b"]);
    }
}
