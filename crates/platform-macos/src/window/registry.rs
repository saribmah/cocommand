use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::{WindowInfo, WindowSnapshot};

const SNAPSHOT_TTL: Duration = Duration::from_secs(5);

struct WindowSnapshotEntry {
    created_at: Instant,
    windows: Vec<WindowInfo>,
}

struct WindowSnapshotStore {
    next_id: u64,
    snapshots: HashMap<u64, WindowSnapshotEntry>,
}

impl WindowSnapshotStore {
    fn new() -> Self {
        Self {
            next_id: 1,
            snapshots: HashMap::new(),
        }
    }

    fn store_snapshot(&mut self, windows: Vec<WindowInfo>) -> WindowSnapshot {
        self.prune();
        let snapshot_id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.snapshots.insert(
            snapshot_id,
            WindowSnapshotEntry {
                created_at: Instant::now(),
                windows: windows.clone(),
            },
        );
        WindowSnapshot {
            snapshot_id,
            windows,
        }
    }

    fn get_snapshot(&mut self, snapshot_id: u64) -> Option<WindowSnapshot> {
        self.prune();
        self.snapshots
            .get(&snapshot_id)
            .map(|entry| WindowSnapshot {
                snapshot_id,
                windows: entry.windows.clone(),
            })
    }

    fn prune(&mut self) {
        let now = Instant::now();
        self.snapshots
            .retain(|_, entry| now.duration_since(entry.created_at) <= SNAPSHOT_TTL);
    }
}

static WINDOW_SNAPSHOTS: OnceLock<Mutex<WindowSnapshotStore>> = OnceLock::new();

pub fn store_snapshot(windows: Vec<WindowInfo>) -> Result<WindowSnapshot, String> {
    let store = WINDOW_SNAPSHOTS.get_or_init(|| Mutex::new(WindowSnapshotStore::new()));
    let mut store = store
        .lock()
        .map_err(|_| "window snapshot store poisoned".to_string())?;
    Ok(store.store_snapshot(windows))
}

pub fn get_snapshot(snapshot_id: u64) -> Option<WindowSnapshot> {
    let store = WINDOW_SNAPSHOTS.get_or_init(|| Mutex::new(WindowSnapshotStore::new()));
    let mut store = store.lock().ok()?;
    store.get_snapshot(snapshot_id)
}
