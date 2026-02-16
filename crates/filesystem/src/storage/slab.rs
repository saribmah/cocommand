//! Memory-mapped slab allocator.
//!
//! This module provides a disk-backed slab allocator that stores entries in a
//! memory-mapped temporary file. This allows the OS to page large indexes in
//! and out of memory, enabling efficient handling of millions of filesystem
//! entries without exhausting heap memory.

use std::fmt;
use std::io;
use std::marker::PhantomData;
use std::mem::{self, MaybeUninit};
use std::num::NonZeroUsize;
use std::ops::{Index, IndexMut};
use std::slice;

use memmap2::{MmapMut, MmapOptions};
use serde::de::{Deserialize, Deserializer, Error as DeError, MapAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, Serializer};
use tempfile::NamedTempFile;

use super::entry::Entry;

/// Initial number of slots to allocate.
const INITIAL_SLOTS: NonZeroUsize = match NonZeroUsize::new(1024) {
    Some(n) => n,
    None => unreachable!(),
};

/// Disk-backed slab that keeps node payloads in a temporary mmap file so the OS
/// can page the largest structure in and out of memory.
pub struct Slab<T> {
    /// Anonymous temporary file that owns the on-disk backing storage.
    file: NamedTempFile,

    /// Memory-mapped view of the file; stores the raw `Entry<T>` array.
    entries: MmapMut,

    /// Number of slots currently mapped.
    entries_capacity: NonZeroUsize,

    /// Number of slots that have been initialized.
    entries_len: usize,

    /// Logical element count (occupied slots only).
    len: usize,

    /// Head of the freelist (index of the next available slot).
    next: usize,

    _marker: PhantomData<T>,
}

impl<T> Slab<T> {
    /// Creates a new empty slab with default capacity.
    pub fn new() -> io::Result<Self> {
        Self::with_capacity(INITIAL_SLOTS)
    }

    /// Creates a new slab with the specified initial capacity.
    pub(super) fn with_capacity(capacity: NonZeroUsize) -> io::Result<Self> {
        let mut file = NamedTempFile::new()?;
        let mmap = Self::map_file(&mut file, capacity)?;
        Ok(Self {
            file,
            entries: mmap,
            len: 0,
            next: 0,
            entries_capacity: capacity,
            entries_len: 0,
            _marker: PhantomData,
        })
    }

    /// Maps the file with the given slot capacity.
    fn map_file(file: &mut NamedTempFile, slots: NonZeroUsize) -> io::Result<MmapMut> {
        let bytes = (slots.get() as u64).saturating_mul(mem::size_of::<Entry<T>>() as u64);
        file.as_file_mut().set_len(bytes)?;
        unsafe { MmapOptions::new().map_mut(file.as_file()) }
    }

    /// Ensures the mmap can host at least `min_capacity` entries.
    ///
    /// Uses doubling strategy to keep amortized O(1) inserts.
    #[inline]
    fn ensure_capacity(&mut self, min_capacity: NonZeroUsize) -> io::Result<()> {
        if min_capacity <= self.entries_capacity {
            return Ok(());
        }
        let mut new_capacity = self.entries_capacity;
        while new_capacity < min_capacity {
            new_capacity = new_capacity.saturating_mul(NonZeroUsize::new(2).unwrap());
        }
        self.remap(new_capacity)
    }

    /// Flushes dirty pages and remaps the file with new capacity.
    #[inline]
    fn remap(&mut self, new_capacity: NonZeroUsize) -> io::Result<()> {
        assert!(new_capacity.get() >= self.entries_len);
        self.entries.flush()?;
        self.entries = Self::map_file(&mut self.file, new_capacity)?;
        self.entries_capacity = new_capacity;
        Ok(())
    }

    /// Returns a slice view of the entries.
    fn entries(&self) -> &[MaybeUninit<Entry<T>>] {
        unsafe {
            slice::from_raw_parts(
                self.entries.as_ptr().cast::<MaybeUninit<Entry<T>>>(),
                self.entries_capacity.get(),
            )
        }
    }

    /// Returns a mutable slice view of the entries.
    fn entries_mut(&mut self) -> &mut [MaybeUninit<Entry<T>>] {
        unsafe {
            slice::from_raw_parts_mut(
                self.entries.as_mut_ptr().cast::<MaybeUninit<Entry<T>>>(),
                self.entries_capacity.get(),
            )
        }
    }

    /// Gets a reference to an entry by index.
    fn entry(&self, index: usize) -> Option<&Entry<T>> {
        (index < self.entries_len)
            .then(|| unsafe { self.entries().get_unchecked(index).assume_init_ref() })
    }

    /// Gets a mutable reference to an entry by index.
    fn entry_mut(&mut self, index: usize) -> Option<&mut Entry<T>> {
        (index < self.entries_len).then(|| unsafe {
            self.entries_mut()
                .get_unchecked_mut(index)
                .assume_init_mut()
        })
    }

    /// Writes an entry at the given index.
    fn write_entry(&mut self, index: usize, entry: Entry<T>) {
        unsafe {
            self.entries_mut().get_unchecked_mut(index).write(entry);
        }
    }

    /// Grows to the next power-of-two capacity.
    fn grow(&mut self) -> io::Result<()> {
        let desired = self
            .entries_capacity
            .saturating_mul(NonZeroUsize::new(2).unwrap());
        self.ensure_capacity(desired)
    }

    /// Inserts a value, returning its stable index.
    pub fn insert(&mut self, value: T) -> io::Result<usize> {
        let key = self.next;
        self.insert_at(key, value)?;
        Ok(key)
    }

    /// Core insertion routine.
    fn insert_at(&mut self, key: usize, value: T) -> io::Result<()> {
        if key == self.entries_len {
            // Appending to end
            if self.entries_len == self.entries_capacity.get() {
                self.grow()?;
            }
            self.write_entry(self.entries_len, Entry::Occupied(value));
            self.entries_len += 1;
            self.next = self.entries_len;
        } else {
            // Reusing a vacant slot from the freelist
            let entry = self
                .entry_mut(key)
                .expect("slot must exist when reusing keys");
            let next_free = match entry {
                Entry::Vacant(next) => *next,
                Entry::Occupied(_) => unreachable!("slot unexpectedly occupied"),
            };
            *entry = Entry::Occupied(value);
            self.next = next_free;
        }
        self.len += 1;
        Ok(())
    }

    /// Gets a reference to the value at `index`.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.entry(index).and_then(|entry| match entry {
            Entry::Occupied(value) => Some(value),
            Entry::Vacant(_) => None,
        })
    }

    /// Gets a mutable reference to the value at `index`.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.entry_mut(index).and_then(|entry| match entry {
            Entry::Occupied(value) => Some(value),
            Entry::Vacant(_) => None,
        })
    }

    /// Removes the value at `index` if it exists, returning it.
    pub fn try_remove(&mut self, index: usize) -> Option<T> {
        let next_free = self.next;
        if let Some(entry) = self.entry_mut(index) {
            let prev = mem::replace(entry, Entry::Vacant(next_free));
            if let Entry::Occupied(value) = prev {
                self.len = self.len.saturating_sub(1);
                self.next = index;
                return Some(value);
            } else {
                *entry = prev;
            }
        }
        None
    }

    /// Returns the number of occupied slots.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if the slab is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns an iterator over occupied entries.
    pub fn iter(&self) -> SlabIter<'_, T> {
        SlabIter {
            slab: self,
            index: 0,
        }
    }

    // Builder helpers for deserialization

    /// Reserves a slot at the given index, materializing vacant slots as needed.
    pub(super) fn builder_reserve_slot(&mut self, index: usize) -> io::Result<()> {
        self.ensure_capacity(NonZeroUsize::new(index.saturating_add(1)).unwrap())?;
        while self.entries_len <= index {
            self.write_entry(self.entries_len, Entry::Vacant(self.next));
            self.entries_len += 1;
        }
        Ok(())
    }

    /// Gets a mutable reference to an entry (for builder use).
    pub(super) fn builder_entry_mut(&mut self, index: usize) -> &mut Entry<T> {
        self.entry_mut(index).expect("builder ensured slot exists")
    }

    /// Returns the number of initialized slots.
    pub(super) fn builder_slots(&self) -> usize {
        self.entries_len
    }

    /// Sets the freelist head.
    pub(super) fn builder_set_next(&mut self, next: usize) {
        self.next = next;
    }

    /// Increments the logical length.
    pub(super) fn builder_increment_len(&mut self) {
        self.len += 1;
    }
}

impl<T> Drop for Slab<T> {
    fn drop(&mut self) {
        // Drop all initialized entries
        for i in 0..self.entries_len {
            unsafe {
                self.entries_mut().get_unchecked_mut(i).assume_init_drop();
            }
        }
        let _ = self.entries.flush();
    }
}

impl<T> Index<usize> for Slab<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("invalid slab index")
    }
}

impl<T> IndexMut<usize> for Slab<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).expect("invalid slab index")
    }
}

impl<T> fmt::Debug for Slab<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Slab")
            .field("len", &self.len)
            .field("next", &self.next)
            .field("slots", &self.entries_len)
            .field("capacity", &self.entries_capacity)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Slab Iterator
// ---------------------------------------------------------------------------

/// Iterator over occupied entries in a Slab.
pub struct SlabIter<'a, T> {
    slab: &'a Slab<T>,
    index: usize,
}

impl<'a, T> Iterator for SlabIter<'a, T> {
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.slab.entries_len {
            let idx = self.index;
            self.index += 1;
            if let Some(value) = self.slab.get(idx) {
                return Some((idx, value));
            }
        }
        None
    }
}

impl<'a, T> IntoIterator for &'a Slab<T> {
    type Item = (usize, &'a T);
    type IntoIter = SlabIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// ---------------------------------------------------------------------------
// Slab Serialization
// ---------------------------------------------------------------------------

impl<T> Serialize for Slab<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (key, value) in self {
            map.serialize_key(&key)?;
            map.serialize_value(value)?;
        }
        map.end()
    }
}

struct SlabVisitor<T>(PhantomData<T>);

impl<'de, T> Visitor<'de> for SlabVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = Slab<T>;

    fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "a map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let size = map.size_hint().unwrap_or_default();
        let size = NonZeroUsize::new(size).unwrap_or(INITIAL_SLOTS);
        let mut slab = Slab::with_capacity(size).map_err(A::Error::custom)?;

        while let Some((key, value)) = map.next_entry::<usize, T>()? {
            // Reserve slots up to the key
            slab.builder_reserve_slot(key).map_err(A::Error::custom)?;
            let entry = slab.builder_entry_mut(key);
            match entry {
                Entry::Occupied(existing) => {
                    *existing = value;
                }
                Entry::Vacant(_) => {
                    *entry = Entry::Occupied(value);
                    slab.builder_increment_len();
                }
            }
        }

        // Rebuild the freelist
        let mut next = slab.builder_slots();
        for idx in (0..slab.builder_slots()).rev() {
            let entry = slab.builder_entry_mut(idx);
            if matches!(entry, Entry::Vacant(_)) {
                *entry = Entry::Vacant(next);
                next = idx;
            }
        }
        slab.builder_set_next(next);

        Ok(slab)
    }
}

impl<'de, T> Deserialize<'de> for Slab<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(SlabVisitor(PhantomData))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slab_basic_operations() {
        let mut slab = Slab::<i32>::new().expect("slab creation should succeed");
        assert!(slab.is_empty());
        assert_eq!(slab.len(), 0);

        let idx0 = slab.insert(10).expect("insert should succeed");
        let idx1 = slab.insert(20).expect("insert should succeed");
        let idx2 = slab.insert(30).expect("insert should succeed");

        assert_eq!(slab.len(), 3);
        assert_eq!(slab.get(idx0), Some(&10));
        assert_eq!(slab.get(idx1), Some(&20));
        assert_eq!(slab.get(idx2), Some(&30));

        // Remove middle element
        let removed = slab.try_remove(idx1);
        assert_eq!(removed, Some(20));
        assert_eq!(slab.len(), 2);
        assert_eq!(slab.get(idx1), None);

        // Insert reuses freed slot
        let idx3 = slab.insert(40).expect("insert should succeed");
        assert_eq!(idx3, idx1); // Should reuse the freed slot
        assert_eq!(slab.get(idx3), Some(&40));
    }

    #[test]
    fn slab_index_access() {
        let mut slab = Slab::<&str>::new().expect("slab creation should succeed");
        let idx = slab.insert("hello").expect("insert should succeed");

        assert_eq!(slab[idx], "hello");
        slab[idx] = "world";
        assert_eq!(slab[idx], "world");
    }

    #[test]
    fn slab_iteration() {
        let mut slab = Slab::<i32>::new().expect("slab creation should succeed");
        slab.insert(1).unwrap();
        slab.insert(2).unwrap();
        slab.insert(3).unwrap();

        let items: Vec<_> = slab.iter().collect();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], (0, &1));
        assert_eq!(items[1], (1, &2));
        assert_eq!(items[2], (2, &3));
    }

    #[test]
    fn slab_grows_automatically() {
        let mut slab = Slab::<i32>::new().expect("slab creation should succeed");

        // Insert more than initial capacity
        for i in 0..2000 {
            slab.insert(i).expect("insert should succeed");
        }

        assert_eq!(slab.len(), 2000);
        for i in 0..2000 {
            assert_eq!(slab.get(i), Some(&(i as i32)));
        }
    }

    #[test]
    fn slab_serialization_roundtrip() {
        let mut slab = Slab::<i32>::new().expect("slab creation should succeed");
        slab.insert(10).unwrap();
        slab.insert(20).unwrap();
        let idx = slab.insert(30).unwrap();
        slab.try_remove(1); // Create a gap

        // Serialize with postcard
        let bytes = postcard::to_stdvec(&slab).expect("serialization should succeed");

        // Deserialize
        let restored: Slab<i32> =
            postcard::from_bytes(&bytes).expect("deserialization should succeed");

        assert_eq!(restored.len(), 2);
        assert_eq!(restored.get(0), Some(&10));
        assert_eq!(restored.get(1), None); // Gap
        assert_eq!(restored.get(idx), Some(&30));
    }
}
