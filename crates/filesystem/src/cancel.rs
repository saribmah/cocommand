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
