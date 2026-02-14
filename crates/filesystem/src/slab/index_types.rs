//! Slab index types for type-safe indexing.

use serde::de::{Deserializer, Error as DeError};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use thin_vec::ThinVec;

/// A compact 32-bit index into the slab.
///
/// Using u32 limits us to ~4 billion entries, which is sufficient for
/// filesystem indexing. The u32::MAX value is reserved for `OptionSlabIndex`
/// and used as an invalid/sentinel value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SlabIndex(u32);

impl SlabIndex {
    /// Invalid index sentinel value (u32::MAX).
    ///
    /// Used to indicate "no valid index" in contexts where Option is not used.
    pub const INVALID: Self = Self(u32::MAX);

    /// Creates a new SlabIndex from a usize.
    ///
    /// # Panics
    /// Panics if `index >= u32::MAX` (reserved for None sentinel).
    #[inline]
    pub fn new(index: usize) -> Self {
        assert!(
            index < u32::MAX as usize,
            "slab index must be less than u32::MAX"
        );
        Self(index as u32)
    }

    /// Returns the index as a usize.
    #[inline]
    pub fn get(&self) -> usize {
        self.0 as usize
    }
}

impl Serialize for SlabIndex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SlabIndex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        if value == u32::MAX {
            return Err(D::Error::custom("SlabIndex cannot be u32::MAX"));
        }
        Ok(Self(value))
    }
}

/// An optional slab index using u32::MAX as the None sentinel.
///
/// This provides a space-efficient Option<SlabIndex> that fits in 4 bytes
/// instead of 8 (due to Option's discriminant).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct OptionSlabIndex(u32);

impl OptionSlabIndex {
    /// Creates a None value.
    #[inline]
    pub fn none() -> Self {
        Self(u32::MAX)
    }

    /// Creates a Some value from a SlabIndex.
    #[inline]
    pub fn some(index: SlabIndex) -> Self {
        Self(index.0)
    }

    /// Creates from an Option<SlabIndex>.
    #[inline]
    pub fn from_option(index: Option<SlabIndex>) -> Self {
        index.map_or(Self::none(), Self::some)
    }

    /// Converts to an Option<SlabIndex>.
    #[inline]
    pub fn to_option(self) -> Option<SlabIndex> {
        if self.0 == u32::MAX {
            None
        } else {
            Some(SlabIndex(self.0))
        }
    }
}

impl Serialize for OptionSlabIndex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OptionSlabIndex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(u32::deserialize(deserializer)?))
    }
}

impl Default for OptionSlabIndex {
    fn default() -> Self {
        Self::none()
    }
}

// ---------------------------------------------------------------------------
// SortedSlabIndices
// ---------------------------------------------------------------------------

/// A sorted collection of slab indices using ThinVec for memory efficiency.
///
/// This type wraps `ThinVec<SlabIndex>` and provides:
/// - Memory efficiency: Empty collections use only a single null pointer (8 bytes)
///   compared to `Vec<T>` which uses 24 bytes even when empty.
/// - Sorted order: Indices are maintained in sorted order for deterministic results.
///
/// Mirrors Cardinal's `SortedSlabIndices` from `name_index.rs`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct SortedSlabIndices {
    indices: ThinVec<SlabIndex>,
}

impl SortedSlabIndices {
    /// Creates a collection with a single index.
    #[inline]
    pub fn with_single(index: SlabIndex) -> Self {
        Self {
            indices: ThinVec::from_iter([index]),
        }
    }

    /// Returns the number of indices.
    #[inline]
    #[allow(dead_code)] // Used by tests
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    /// Returns true if empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Iterates over the indices.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &SlabIndex> {
        self.indices.iter()
    }

    /// Inserts an index in sorted order, avoiding duplicates.
    ///
    /// The `path_fn` closure is used to get the sort key (path) for each index.
    /// This ensures indices are sorted by their full path for deterministic results.
    pub fn insert_sorted<F>(&mut self, index: SlabIndex, path_fn: F)
    where
        F: Fn(SlabIndex) -> Option<String>,
    {
        let Some(target_path) = path_fn(index) else {
            return;
        };

        match self.indices.binary_search_by(|existing| {
            path_fn(*existing)
                .expect("existing index must resolve to a path")
                .cmp(&target_path)
        }) {
            Ok(_) => {} // Already exists, skip
            Err(pos) => self.indices.insert(pos, index),
        }
    }

    /// Inserts an index without checking sort order.
    ///
    /// # Safety
    /// The caller must ensure indices are inserted in path-sorted order.
    /// This is safe when bulk-loading from path-sorted entries.
    #[inline]
    pub unsafe fn push_ordered(&mut self, index: SlabIndex) {
        self.indices.push(index);
    }

    /// Removes an index, returning true if it was present.
    pub fn remove(&mut self, index: SlabIndex) -> bool {
        if let Some(pos) = self.indices.iter().position(|&existing| existing == index) {
            self.indices.remove(pos);
            true
        } else {
            false
        }
    }
}

impl FromIterator<SlabIndex> for SortedSlabIndices {
    fn from_iter<I: IntoIterator<Item = SlabIndex>>(iter: I) -> Self {
        Self {
            indices: ThinVec::from_iter(iter),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slab_index_types() {
        let idx = SlabIndex::new(100);
        assert_eq!(idx.get(), 100);

        let opt_none = OptionSlabIndex::none();
        assert_eq!(opt_none.to_option(), None);

        let opt_some = OptionSlabIndex::some(idx);
        assert_eq!(opt_some.to_option(), Some(idx));

        let opt_from = OptionSlabIndex::from_option(Some(idx));
        assert_eq!(opt_from.to_option(), Some(idx));

        let opt_from_none = OptionSlabIndex::from_option(None);
        assert_eq!(opt_from_none.to_option(), None);
    }

    #[test]
    fn sorted_slab_indices_with_single() {
        let idx = SlabIndex::new(42);
        let indices = SortedSlabIndices::with_single(idx);
        assert!(!indices.is_empty());
        let collected: Vec<_> = indices.iter().copied().collect();
        assert_eq!(collected, vec![idx]);
    }

    #[test]
    fn sorted_slab_indices_from_iter() {
        let idx1 = SlabIndex::new(1);
        let idx2 = SlabIndex::new(2);
        let idx3 = SlabIndex::new(3);

        let indices: SortedSlabIndices = vec![idx1, idx2, idx3].into_iter().collect();
        assert!(!indices.is_empty());
        let collected: Vec<_> = indices.iter().copied().collect();
        assert_eq!(collected, vec![idx1, idx2, idx3]);
    }

    #[test]
    fn sorted_slab_indices_memory_efficiency() {
        // ThinVec should use only 8 bytes for an empty vector (one pointer)
        // vs Vec's 24 bytes (ptr + len + capacity)
        assert_eq!(std::mem::size_of::<SortedSlabIndices>(), 8);
        assert_eq!(std::mem::size_of::<Vec<SlabIndex>>(), 24);
    }

    #[test]
    fn sorted_slab_indices_remove() {
        let idx1 = SlabIndex::new(10);
        let idx2 = SlabIndex::new(20);

        let mut indices: SortedSlabIndices = vec![idx1, idx2].into_iter().collect();

        assert!(indices.remove(idx1));
        // Removing non-existent should return false
        assert!(!indices.remove(idx1));
        assert!(indices.remove(idx2));
        assert!(indices.is_empty());
    }
}
