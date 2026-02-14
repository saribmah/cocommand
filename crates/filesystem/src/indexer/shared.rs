//! Shared root index state.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use super::build::{FlushSignal, IndexBuildProgress};
use super::data::RootIndexData;

/// Shared state for a root index.
#[derive(Debug)]
pub struct SharedRootIndex {
    /// Root path being indexed.
    pub root: PathBuf,
    /// Whether the root is a directory.
    pub root_is_dir: bool,
    /// Paths to ignore during indexing.
    pub ignored_roots: Vec<PathBuf>,
    /// Path to the cache file.
    pub cache_path: PathBuf,
    /// Signal for the flush worker.
    pub flush_signal: Arc<FlushSignal>,
    /// Current build state (atomic for lock-free reads).
    pub build_state: AtomicU8,
    /// Build progress tracking.
    pub build_progress: IndexBuildProgress,
    /// Build generation counter (for cancellation detection).
    pub build_generation: AtomicU64,
    /// Cancellation flag for the current build.
    pub build_cancel: Mutex<Option<Arc<std::sync::atomic::AtomicBool>>>,
    /// Last build error message.
    pub build_last_error: Mutex<Option<String>>,
    /// Pending path changes to apply after build completes.
    pub pending_changes: Mutex<Vec<PathBuf>>,
    /// The actual index data.
    pub data: RwLock<RootIndexData>,
    /// Last FSEvents event ID (macOS).
    pub last_event_id: AtomicU64,
    /// Count of full rescans performed. Incremented when FS events trigger a full rescan.
    /// Used by the UI to detect when search results may be stale and need refresh.
    /// Matches Cardinal's `rescan_count` field in SearchCache.
    pub rescan_count: AtomicU64,
}

impl SharedRootIndex {
    /// Creates a new SharedRootIndex for testing.
    #[cfg(test)]
    pub fn for_tests(
        root: PathBuf,
        root_is_dir: bool,
        ignored_roots: Vec<PathBuf>,
        cache_path: PathBuf,
    ) -> Self {
        Self {
            root,
            root_is_dir,
            ignored_roots,
            cache_path,
            flush_signal: Arc::new(FlushSignal::default()),
            build_state: AtomicU8::new(0),
            build_progress: IndexBuildProgress::default(),
            build_generation: AtomicU64::new(0),
            build_cancel: Mutex::new(None),
            build_last_error: Mutex::new(None),
            pending_changes: Mutex::new(Vec::new()),
            data: RwLock::new(RootIndexData::default()),
            last_event_id: AtomicU64::new(0),
            rescan_count: AtomicU64::new(0),
        }
    }

    /// Increments the rescan count. Called when FSEvents trigger a full rescan.
    pub fn increment_rescan_count(&self) {
        self.rescan_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns the current rescan count.
    pub fn rescan_count(&self) -> u64 {
        self.rescan_count.load(Ordering::Relaxed)
    }
}
