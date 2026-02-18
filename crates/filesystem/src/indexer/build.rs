//! Index build state and progress tracking.

use std::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Index build state.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum IndexBuildState {
    Idle = 0,
    Building = 1,
    Ready = 2,
    Error = 3,
    Updating = 4,
}

impl IndexBuildState {
    /// Loads the state from an atomic.
    pub fn load(atomic: &AtomicU8) -> Self {
        match atomic.load(Ordering::Relaxed) {
            1 => Self::Building,
            2 => Self::Ready,
            3 => Self::Error,
            4 => Self::Updating,
            _ => Self::Idle,
        }
    }

    /// Returns the state as a string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Building => "building",
            Self::Ready => "ready",
            Self::Error => "error",
            Self::Updating => "updating",
        }
    }
}

/// Progress tracking for index building.
#[derive(Debug, Default)]
pub struct IndexBuildProgress {
    pub scanned_files: AtomicUsize,
    pub scanned_dirs: AtomicUsize,
    pub errors: AtomicUsize,
    pub started_at: AtomicU64,
    pub last_update_at: AtomicU64,
    pub finished_at: AtomicU64,
}

impl IndexBuildProgress {
    /// Creates progress that is already complete (for cache loads).
    pub fn new_ready(saved_at: u64) -> Self {
        Self {
            scanned_files: AtomicUsize::new(0),
            scanned_dirs: AtomicUsize::new(0),
            errors: AtomicUsize::new(0),
            started_at: AtomicU64::new(saved_at),
            last_update_at: AtomicU64::new(saved_at),
            finished_at: AtomicU64::new(saved_at),
        }
    }

    /// Resets progress for a new build.
    pub fn reset_for_build(&self, started_at: u64) {
        self.scanned_files.store(0, Ordering::Relaxed);
        self.scanned_dirs.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
        self.started_at.store(started_at, Ordering::Relaxed);
        self.last_update_at.store(started_at, Ordering::Relaxed);
        self.finished_at.store(0, Ordering::Relaxed);
    }

    /// Takes a snapshot of the progress values.
    pub fn snapshot(&self) -> ProgressSnapshot {
        ProgressSnapshot {
            scanned_files: self.scanned_files.load(Ordering::Relaxed),
            scanned_dirs: self.scanned_dirs.load(Ordering::Relaxed),
            started_at: zero_to_none(self.started_at.load(Ordering::Relaxed)),
            last_update_at: zero_to_none(self.last_update_at.load(Ordering::Relaxed)),
            finished_at: zero_to_none(self.finished_at.load(Ordering::Relaxed)),
        }
    }
}

/// A snapshot of build progress values.
#[derive(Debug, Clone)]
pub struct ProgressSnapshot {
    pub scanned_files: usize,
    pub scanned_dirs: usize,
    pub started_at: Option<u64>,
    pub last_update_at: Option<u64>,
    pub finished_at: Option<u64>,
}

/// Flush poll interval — how often we check whether a flush is due.
pub const INDEX_FLUSH_POLL: Duration = Duration::from_secs(10);

/// Idle flush threshold — flush only after this long with no search activity.
/// Matches Cardinal's IDLE_FLUSH_INTERVAL (5 minutes).
pub const INDEX_FLUSH_IDLE: Duration = Duration::from_secs(5 * 60);

/// Safety-net max delay — flush even if searches keep coming, to avoid data loss.
pub const INDEX_FLUSH_MAX_DELAY: Duration = Duration::from_secs(10 * 60);

/// Returns the current Unix timestamp in seconds.
pub fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}

/// Converts 0 to None for optional timestamps.
pub fn zero_to_none(value: u64) -> Option<u64> {
    if value == 0 {
        None
    } else {
        Some(value)
    }
}
