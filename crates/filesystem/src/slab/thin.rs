//! ThinSlab - High-level wrapper with SlabIndex API.

use std::io;
use std::ops::{Index, IndexMut};

use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};

use super::index_types::SlabIndex;
use super::mmap::{Slab, SlabIter};

/// A wrapper around `Slab<T>` that uses `SlabIndex` for type safety.
///
/// ThinSlab provides the same functionality as Slab but with a type-safe
/// index type that prevents mixing up indices from different slabs.
#[derive(Debug)]
pub struct ThinSlab<T>(Slab<T>);

impl<T> Default for ThinSlab<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ThinSlab<T> {
    /// Constructs a ThinSlab, panicking if mmap initialization fails.
    pub fn new() -> Self {
        Self::try_new().expect("ThinSlab::new failed to initialize memory-mapped slab")
    }

    /// Constructs a ThinSlab, propagating I/O failures to the caller.
    pub fn try_new() -> io::Result<Self> {
        Slab::new().map(Self)
    }

    /// Inserts a value, returning its index.
    ///
    /// # Panics
    /// Panics if the backing slab fails to grow due to I/O error.
    pub fn insert(&mut self, value: T) -> SlabIndex {
        self.try_insert(value)
            .expect("ThinSlab::insert failed to grow backing slab")
    }

    /// Inserts a value, propagating any I/O failures.
    pub fn try_insert(&mut self, value: T) -> io::Result<SlabIndex> {
        self.0.insert(value).map(SlabIndex::new)
    }

    /// Gets a reference to the value at `index`.
    pub fn get(&self, index: SlabIndex) -> Option<&T> {
        self.0.get(index.get())
    }

    /// Gets a mutable reference to the value at `index`.
    pub fn get_mut(&mut self, index: SlabIndex) -> Option<&mut T> {
        self.0.get_mut(index.get())
    }

    /// Removes the value at `index`, returning it if present.
    pub fn try_remove(&mut self, index: SlabIndex) -> Option<T> {
        self.0.try_remove(index.get())
    }

    /// Returns the number of occupied slots.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if the slab is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns an iterator over occupied entries.
    pub fn iter(&self) -> ThinSlabIter<'_, T> {
        ThinSlabIter(self.0.iter())
    }
}

impl<T> Index<SlabIndex> for ThinSlab<T> {
    type Output = T;

    fn index(&self, index: SlabIndex) -> &Self::Output {
        &self.0[index.get()]
    }
}

impl<T> IndexMut<SlabIndex> for ThinSlab<T> {
    fn index_mut(&mut self, index: SlabIndex) -> &mut Self::Output {
        &mut self.0[index.get()]
    }
}

impl<T: Serialize> Serialize for ThinSlab<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for ThinSlab<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Slab::deserialize(deserializer).map(Self)
    }
}

/// Iterator over entries in a ThinSlab.
pub struct ThinSlabIter<'a, T>(SlabIter<'a, T>);

impl<'a, T> Iterator for ThinSlabIter<'a, T> {
    type Item = (SlabIndex, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|(idx, value)| (SlabIndex::new(idx), value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thin_slab_basic_operations() {
        let mut slab = ThinSlab::<i32>::try_new().expect("creation should succeed");
        assert!(slab.is_empty());

        let idx = slab.insert(42);
        assert_eq!(slab.get(idx), Some(&42));
        assert_eq!(slab[idx], 42);
        assert_eq!(slab.len(), 1);

        let removed = slab.try_remove(idx);
        assert_eq!(removed, Some(42));
        assert!(slab.is_empty());
    }
}
