//! String interning pool for filesystem entry names.
//!
//! This module provides a `NamePool` that stores unique strings and returns
//! stable references to them. This reduces memory usage when many entries
//! share the same filename (e.g., "README.md", "Cargo.toml", ".gitignore").
//!
//! ## Global NAME_POOL
//!
//! A global `NAME_POOL` is provided for use during deserialization of `SlabNode`s.

use std::collections::BTreeSet;
use std::sync::{LazyLock, Mutex};

/// Global name pool for filesystem entry names.
///
/// This is used during deserialization to re-intern strings from persisted cache.
/// The pool is never dropped, ensuring all `&'static str` references remain valid.
pub static NAME_POOL: LazyLock<NamePool> = LazyLock::new(NamePool::new);

/// A pool that interns strings, storing each unique string exactly once.
///
/// When you push a string into the pool, it returns a reference that remains
/// valid for the lifetime of the pool. Duplicate strings return the same
/// reference, saving memory.
pub struct NamePool {
    inner: Mutex<BTreeSet<Box<str>>>,
}

impl std::fmt::Debug for NamePool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len = self.inner.lock().map(|guard| guard.len()).unwrap_or(0);
        f.debug_struct("NamePool").field("len", &len).finish()
    }
}

impl Default for NamePool {
    fn default() -> Self {
        Self::new()
    }
}

impl NamePool {
    /// Creates a new empty name pool.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(BTreeSet::new()),
        }
    }

    /// Interns a string into the pool, returning a reference to the stored string.
    ///
    /// If the string already exists in the pool, returns a reference to the
    /// existing copy. If it's new, stores it and returns a reference.
    ///
    /// # Safety
    ///
    /// The returned `&'static str` is safe because:
    /// 1. The `Box<str>` is stored in a `BTreeSet` that never removes elements
    /// 2. The pool itself is never cleared or dropped while references exist
    /// 3. We only hand out references from within the same pool lifetime
    ///
    /// In practice, the NamePool should be stored in a long-lived location
    /// (like a lazy_static or within the index manager) and outlive all
    /// references to its strings.
    pub fn intern(&self, name: &str) -> &'static str {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if !inner.contains(name) {
            inner.insert(name.into());
        }

        let existing = inner.get(name).expect("just inserted or already present");
        // SAFETY: The Box<str> is heap-allocated and never moved or freed.
        // We extend the lifetime to 'static because the pool outlives usage.
        unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                existing.as_ptr(),
                existing.len(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_basic() {
        let pool = NamePool::new();
        let s = pool.intern("hello");
        assert_eq!(s, "hello");
    }

    #[test]
    fn test_intern_deduplication() {
        let pool = NamePool::new();
        let s1 = pool.intern("hello");
        let s2 = pool.intern("hello");
        assert_eq!(s1.as_ptr(), s2.as_ptr()); // Same memory location
    }

    #[test]
    fn test_intern_multiple() {
        let pool = NamePool::new();
        let s1 = pool.intern("foo");
        let s2 = pool.intern("bar");
        let s3 = pool.intern("baz");
        assert_eq!(s1, "foo");
        assert_eq!(s2, "bar");
        assert_eq!(s3, "baz");
    }

    #[test]
    fn test_intern_empty_string() {
        let pool = NamePool::new();
        let s = pool.intern("");
        assert_eq!(s, "");
    }

    #[test]
    fn test_intern_unicode() {
        let pool = NamePool::new();
        let s = pool.intern("こんにちは");
        assert_eq!(s, "こんにちは");
    }

    #[test]
    fn test_many_duplicates() {
        let pool = NamePool::new();
        for _ in 0..100 {
            let s = pool.intern("duplicate");
            assert_eq!(s, "duplicate");
        }
    }
}
