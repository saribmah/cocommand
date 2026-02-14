//! Cancellation tokens for search and indexing operations.
//!
//! This module provides a simple cancellation token that can be used to
//! terminate long-running operations early.
//!
//! ## Sparse Checking
//!
//! For tight loops processing millions of items, `is_cancelled_sparse()`
//! only checks every 65,536 iterations to minimize atomic read overhead.

use std::sync::atomic::{AtomicU64, Ordering};

/// How often long-running loops should check whether execution was cancelled.
/// Using a power of 2 allows efficient modulo via bitwise AND.
pub const CANCEL_CHECK_INTERVAL: usize = 0x10000; // 65,536

/// Tracks the active search version for cancellation.
///
/// When a new search starts, call `next_version()` to get a new version number.
/// Previous searches with older versions will be cancelled when they check
/// their `CancellationToken`.
#[derive(Debug, Default)]
pub struct SearchVersionTracker {
    active_version: AtomicU64,
}

impl SearchVersionTracker {
    /// Creates a new search version tracker.
    pub fn new() -> Self {
        Self {
            active_version: AtomicU64::new(0),
        }
    }

    /// Increments the active version and returns the new version number.
    ///
    /// This effectively cancels any in-flight searches using older versions.
    pub fn next_version(&self) -> u64 {
        self.active_version.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Marks a caller-provided version as active if it is newer than the
    /// currently active version.
    ///
    /// Returns the resulting active version after the update attempt.
    pub fn activate_version(&self, version: u64) -> u64 {
        let mut current = self.active_version.load(Ordering::SeqCst);
        loop {
            if version <= current {
                return current;
            }
            match self.active_version.compare_exchange(
                current,
                version,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return version,
                Err(observed) => current = observed,
            }
        }
    }

    /// Returns the current active version without incrementing.
    pub fn current_version(&self) -> u64 {
        self.active_version.load(Ordering::SeqCst)
    }

    /// Creates a cancellation token for the given version.
    ///
    /// The token will report as cancelled if the active version has moved past
    /// the given version.
    ///
    /// # Safety
    /// This uses a static reference trick - the tracker must outlive all tokens.
    /// In practice, the tracker lives in FileSystemIndexManager which is long-lived.
    pub fn token_for_version(&self, version: u64) -> CancellationToken {
        // SAFETY: We need a 'static reference for the token. The tracker is owned
        // by FileSystemIndexManager which lives for the app lifetime. We use a raw
        // pointer cast to achieve this - the caller must ensure the tracker outlives
        // all tokens (which is guaranteed by the manager's lifetime).
        let static_ref: &'static AtomicU64 =
            unsafe { &*(&self.active_version as *const AtomicU64) };
        CancellationToken {
            active_version: static_ref,
            version,
        }
    }
}

/// A cancellation token for terminating long-running operations.
#[derive(Clone, Copy, Debug)]
pub struct CancellationToken {
    /// Reference to the atomic holding the active version.
    active_version: &'static AtomicU64,
    /// The version this token was created with.
    version: u64,
}

impl CancellationToken {
    /// Creates a cancellation token that is never cancelled.
    ///
    /// Useful for tests or operations that should not be interruptible.
    #[inline]
    pub fn noop() -> Self {
        static NOOP: AtomicU64 = AtomicU64::new(0);
        Self {
            version: 0,
            active_version: &NOOP,
        }
    }

    /// Checks if this token is still active.
    ///
    /// Returns `Some(())` if still active, `None` if cancelled.
    /// This enables use with the `?` operator for early returns.
    #[inline]
    pub fn is_cancelled(&self) -> Option<()> {
        if self.version != self.active_version.load(Ordering::Relaxed) {
            None
        } else {
            Some(())
        }
    }

    /// Sparse cancellation check - only checks every `CANCEL_CHECK_INTERVAL` iterations.
    ///
    /// This reduces the overhead of atomic reads in tight loops while still
    /// allowing timely cancellation. The maximum latency before noticing
    /// cancellation is ~65,536 iterations.
    #[inline]
    pub fn is_cancelled_sparse(&self, counter: usize) -> Option<()> {
        // Use bitwise AND for efficient power-of-2 modulo
        if counter & (CANCEL_CHECK_INTERVAL - 1) == 0 {
            self.is_cancelled()
        } else {
            Some(())
        }
    }
}

impl Default for CancellationToken {
    /// Default creates a noop token that is never cancelled.
    fn default() -> Self {
        Self::noop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_token_is_never_cancelled() {
        let token = CancellationToken::noop();
        assert!(token.is_cancelled().is_some());
    }

    #[test]
    fn default_is_noop() {
        let token = CancellationToken::default();
        assert!(token.is_cancelled().is_some());
    }
}
