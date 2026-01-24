//! In-memory storage facade combining all sub-store implementations.

use super::clipboard_store::{ClipboardStore, MemoryClipboardStore};
use super::event_log::{EventLog, MemoryEventLog};
use super::kv_store::{KvStore, MemoryKvStore};
use super::snapshot_store::{MemorySnapshotStore, SnapshotStore};
use super::traits::Storage;

/// In-memory storage implementation satisfying all sub-store traits.
#[derive(Debug, Default)]
pub struct MemoryStorage {
    event_log: MemoryEventLog,
    snapshots: MemorySnapshotStore,
    kv: MemoryKvStore,
    clipboard: MemoryClipboardStore,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Storage for MemoryStorage {
    fn event_log(&self) -> &dyn EventLog {
        &self.event_log
    }

    fn event_log_mut(&mut self) -> &mut dyn EventLog {
        &mut self.event_log
    }

    fn snapshots(&self) -> &dyn SnapshotStore {
        &self.snapshots
    }

    fn snapshots_mut(&mut self) -> &mut dyn SnapshotStore {
        &mut self.snapshots
    }

    fn kv(&self) -> &dyn KvStore {
        &self.kv
    }

    fn kv_mut(&mut self) -> &mut dyn KvStore {
        &mut self.kv
    }

    fn clipboard(&self) -> &dyn ClipboardStore {
        &self.clipboard
    }

    fn clipboard_mut(&mut self) -> &mut dyn ClipboardStore {
        &mut self.clipboard
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Event;
    use serde_json::json;
    use std::time::SystemTime;
    use uuid::Uuid;

    fn make_event(text: &str) -> Event {
        Event::UserMessage {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            text: text.to_string(),
        }
    }

    #[test]
    fn satisfies_storage_trait() {
        let storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        assert!(storage.event_log().is_empty());
        assert!(storage.snapshots().load().is_none());
        assert!(storage.kv().get("x").is_none());
        assert!(storage.clipboard().is_empty());
    }

    #[test]
    fn mut_through_facade() {
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());

        storage.event_log_mut().append(make_event("test"));
        assert_eq!(storage.event_log().len(), 1);

        storage.kv_mut().set("k", json!("v"));
        assert_eq!(storage.kv().get("k"), Some(json!("v")));
    }
}
