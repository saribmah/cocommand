//! Index build state and progress tracking.

use std::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

/// Flush signal for the background worker.
#[derive(Debug, Default)]
pub struct FlushSignal {
    state: Mutex<FlushState>,
    condvar: Condvar,
}

#[derive(Debug, Default)]
struct FlushState {
    dirty: bool,
    shutdown: bool,
    first_dirty_at: Option<Instant>,
    last_dirty_at: Option<Instant>,
}

/// Decision returned by the flush worker.
#[derive(Debug)]
pub enum FlushDecision {
    Flush,
    Shutdown,
}

/// Debounce and max delay constants for flushing.
pub const INDEX_FLUSH_DEBOUNCE_MS: u64 = 1200;
pub const INDEX_FLUSH_MAX_DELAY_SECS: u64 = 20;

impl FlushSignal {
    /// Marks the index as dirty, triggering a future flush.
    pub fn mark_dirty(&self) {
        let now = Instant::now();
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => return,
        };
        if !state.dirty {
            state.first_dirty_at = Some(now);
        }
        state.dirty = true;
        state.last_dirty_at = Some(now);
        self.condvar.notify_all();
    }

    /// Requests shutdown of the flush worker.
    pub fn request_shutdown(&self) {
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => return,
        };
        state.shutdown = true;
        self.condvar.notify_all();
    }

    /// Waits for a flush or shutdown decision.
    pub fn wait_for_flush(&self) -> FlushDecision {
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => return FlushDecision::Shutdown,
        };

        loop {
            if !state.dirty {
                if state.shutdown {
                    return FlushDecision::Shutdown;
                }
                state = match self.condvar.wait(state) {
                    Ok(guard) => guard,
                    Err(_) => return FlushDecision::Shutdown,
                };
                continue;
            }

            let now = Instant::now();
            let debounce_deadline = state
                .last_dirty_at
                .unwrap_or(now)
                .checked_add(Duration::from_millis(INDEX_FLUSH_DEBOUNCE_MS))
                .unwrap_or(now);
            let max_deadline = state
                .first_dirty_at
                .unwrap_or(now)
                .checked_add(Duration::from_secs(INDEX_FLUSH_MAX_DELAY_SECS))
                .unwrap_or(now);
            let next_deadline = debounce_deadline.min(max_deadline);

            if state.shutdown || now >= next_deadline {
                state.dirty = false;
                state.first_dirty_at = None;
                state.last_dirty_at = None;
                return FlushDecision::Flush;
            }

            let wait_for = next_deadline
                .checked_duration_since(now)
                .unwrap_or_else(|| Duration::from_millis(1));
            let (next_state, _) = match self.condvar.wait_timeout(state, wait_for) {
                Ok(value) => value,
                Err(_) => return FlushDecision::Shutdown,
            };
            state = next_state;
        }
    }
}

/// Handle for the background flush worker thread.
#[derive(Debug)]
pub struct FlushWorkerHandle {
    pub signal: Arc<FlushSignal>,
    join_handle: Option<JoinHandle<()>>,
}

impl FlushWorkerHandle {
    /// Creates a new flush worker handle.
    pub fn new(signal: Arc<FlushSignal>, join_handle: JoinHandle<()>) -> Self {
        Self {
            signal,
            join_handle: Some(join_handle),
        }
    }
}

impl Drop for FlushWorkerHandle {
    fn drop(&mut self) {
        self.signal.request_shutdown();
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

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
