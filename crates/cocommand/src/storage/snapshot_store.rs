//! Workspace snapshot store trait and in-memory implementation.

use super::types::WorkspaceSnapshot;

/// Workspace snapshot persistence (single-slot, overwrite semantics).
pub trait SnapshotStore: Send + Sync {
    /// Save a workspace snapshot, replacing any previous one.
    fn save(&mut self, snapshot: WorkspaceSnapshot);
    /// Load the stored snapshot, if any.
    fn load(&self) -> Option<WorkspaceSnapshot>;
}

// --- Memory Implementation ---

#[derive(Debug, Default)]
pub(crate) struct MemorySnapshotStore {
    snapshot: Option<WorkspaceSnapshot>,
}

impl SnapshotStore for MemorySnapshotStore {
    fn save(&mut self, snapshot: WorkspaceSnapshot) {
        self.snapshot = Some(snapshot);
    }

    fn load(&self) -> Option<WorkspaceSnapshot> {
        self.snapshot.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::SystemTime;

    #[test]
    fn starts_empty() {
        let store = MemorySnapshotStore::default();
        assert!(store.load().is_none());
    }

    #[test]
    fn save_and_load() {
        let mut store = MemorySnapshotStore::default();
        store.save(WorkspaceSnapshot {
            session_id: "sess-1".to_string(),
            captured_at: SystemTime::now(),
            data: json!({"mode": "idle"}),
        });

        let loaded = store.load().unwrap();
        assert_eq!(loaded.session_id, "sess-1");
    }

    #[test]
    fn overwrites_previous() {
        let mut store = MemorySnapshotStore::default();
        store.save(WorkspaceSnapshot {
            session_id: "first".to_string(),
            captured_at: SystemTime::now(),
            data: json!({}),
        });
        store.save(WorkspaceSnapshot {
            session_id: "second".to_string(),
            captured_at: SystemTime::now(),
            data: json!({}),
        });

        assert_eq!(store.load().unwrap().session_id, "second");
    }
}
