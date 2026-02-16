//! Slab node types for filesystem index entries.
//!
//! This module provides compact node representations for the filesystem index,
//! `slab_node.rs` and `type_and_size.rs` for memory efficiency.

use serde::{Deserialize, Serialize};
use thin_vec::ThinVec;

use super::index_types::{OptionSlabIndex, SlabIndex};

// ---------------------------------------------------------------------------
// Compact metadata encoding
// ---------------------------------------------------------------------------

/// File type enumeration, `fswalk::NodeFileType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum NodeFileType {
    /// Regular file
    File = 0,
    /// Directory
    Dir = 1,
    /// Symbolic link
    Symlink = 2,
    /// Unknown or other file type
    Unknown = 3,
}

impl NodeFileType {
    /// Creates from a u8 value.
    #[inline]
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::File),
            1 => Some(Self::Dir),
            2 => Some(Self::Symlink),
            3 => Some(Self::Unknown),
            _ => None,
        }
    }
}

impl From<std::fs::FileType> for NodeFileType {
    fn from(file_type: std::fs::FileType) -> Self {
        if file_type.is_file() {
            NodeFileType::File
        } else if file_type.is_dir() {
            NodeFileType::Dir
        } else if file_type.is_symlink() {
            NodeFileType::Symlink
        } else {
            NodeFileType::Unknown
        }
    }
}

/// Metadata access state (internal encoding).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum MetadataState {
    /// Metadata is present
    Some = 1,
}

/// Compact encoding of state, file type, and size in a single u64.
///
/// Layout (from high to low bits):
/// - Bits 62-63: State (2 bits)
/// - Bits 60-61: File type (2 bits)
/// - Bits 0-59: Size (60 bits, max ~1 exabyte)
///
/// This encoding saves memory compared to storing these fields separately.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct StateTypeSize(u64);

impl StateTypeSize {
    /// Maximum size that can be stored (2^60 - 1 bytes, ~1 exabyte).
    pub const MAX_SIZE: u64 = (1u64 << 60) - 1;

    /// Creates a value with present metadata.
    #[inline]
    pub fn some(file_type: NodeFileType, size: u64) -> Self {
        Self::new(MetadataState::Some, file_type, size)
    }

    /// Creates a new StateTypeSize with the given components.
    #[inline]
    fn new(state: MetadataState, file_type: NodeFileType, size: u64) -> Self {
        let clamped_size = size.min(Self::MAX_SIZE);
        Self(clamped_size | ((file_type as u64) << 60) | ((state as u64) << 62))
    }

    /// Returns the file type.
    #[inline]
    pub fn file_type(&self) -> NodeFileType {
        NodeFileType::from_u8(((self.0 >> 60) & 0b11) as u8).unwrap_or(NodeFileType::Unknown)
    }

    /// Returns the raw size value.
    #[inline]
    pub fn raw_size(&self) -> u64 {
        self.0 & Self::MAX_SIZE
    }

    /// Returns true if this represents a directory.
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.file_type() == NodeFileType::Dir
    }

    /// Returns true if this represents a regular file.
    #[inline]
    pub fn is_file(&self) -> bool {
        self.file_type() == NodeFileType::File
    }
}

// ---------------------------------------------------------------------------
// Compact node metadata
// ---------------------------------------------------------------------------

/// Compact metadata for a slab node.
///
/// Uses compact encoding to minimize memory usage:
/// - StateTypeSize: 8 bytes (state, type, size combined)
/// - ctime: 4 bytes (Unix timestamp as u32, good until year 2106)
/// - mtime: 4 bytes (Unix timestamp as u32)
///
/// Total: 16 bytes per node (vs. potentially 40+ bytes with full types)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct SlabNodeMetadata {
    /// Combined state, type, and size.
    pub state_type_size: StateTypeSize,
    /// Creation time as Unix timestamp (seconds since epoch).
    /// 0 means not available.
    pub ctime: u32,
    /// Modification time as Unix timestamp (seconds since epoch).
    /// 0 means not available.
    pub mtime: u32,
}

impl SlabNodeMetadata {
    /// Creates metadata from file system metadata.
    pub fn from_fs_metadata(metadata: &std::fs::Metadata) -> Self {
        use std::time::UNIX_EPOCH;

        let file_type = NodeFileType::from(metadata.file_type());
        let size = metadata.len();

        let ctime = metadata
            .created()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0);

        let mtime = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0);

        Self {
            state_type_size: StateTypeSize::some(file_type, size),
            ctime,
            mtime,
        }
    }

    /// Returns the file type.
    #[inline]
    pub fn file_type(&self) -> NodeFileType {
        self.state_type_size.file_type()
    }

    /// Returns true if this is a directory.
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.state_type_size.is_dir()
    }

    /// Returns true if this is a regular file.
    #[inline]
    pub fn is_file(&self) -> bool {
        self.state_type_size.is_file()
    }
}

// ---------------------------------------------------------------------------
// Name and parent reference
// ---------------------------------------------------------------------------

/// Combined name pointer and parent index.
///
/// This stores:
/// - A pointer to the interned filename string (from NamePool)
/// - The length of the filename
/// - The parent node index (or None for root)
///
/// The filename is stored as a raw pointer + length to avoid storing
/// the full String, since names are interned in the NamePool.
#[derive(Debug, Clone, Copy)]
pub struct NameAndParent {
    /// Pointer to the interned string data.
    ptr: *const u8,
    /// Length of the filename (max 256 is typical for filesystems).
    len: u32,
    /// Parent node index, or None for root.
    parent: OptionSlabIndex,
}

// SAFETY: The pointer refers to data in the NamePool which is never freed
// while the index is alive.
unsafe impl Send for NameAndParent {}
unsafe impl Sync for NameAndParent {}

impl NameAndParent {
    /// Creates a new NameAndParent.
    ///
    /// The `name` must be a reference from a NamePool that outlives this value.
    #[inline]
    pub fn new(name: &'static str, parent: OptionSlabIndex) -> Self {
        Self {
            ptr: name.as_ptr(),
            len: name.len() as u32,
            parent,
        }
    }

    /// Returns the filename as a string slice.
    ///
    /// # Safety
    /// The pointer must still be valid (pointing to interned data).
    #[inline]
    pub fn name(&self) -> &'static str {
        unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.ptr, self.len as usize))
        }
    }

    /// Returns the parent node index, if any.
    #[inline]
    pub fn parent(&self) -> Option<SlabIndex> {
        self.parent.to_option()
    }
}

impl std::ops::Deref for NameAndParent {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.name()
    }
}

// Custom serialization for NameAndParent
impl Serialize for NameAndParent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTuple;
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(self.name())?;
        tuple.serialize_element(&self.parent)?;
        tuple.end()
    }
}

// Custom deserialization for NameAndParent - uses global NAME_POOL to re-intern strings
impl<'de> Deserialize<'de> for NameAndParent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error as DeError, SeqAccess, Visitor};
        use std::fmt;

        use super::namepool::NAME_POOL;

        struct NameAndParentVisitor;

        impl<'de> Visitor<'de> for NameAndParentVisitor {
            type Value = NameAndParent;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a tuple of (string, OptionSlabIndex)")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let name: String = seq
                    .next_element()?
                    .ok_or_else(|| A::Error::invalid_length(0, &self))?;
                let parent: OptionSlabIndex = seq
                    .next_element()?
                    .ok_or_else(|| A::Error::invalid_length(1, &self))?;

                // Re-intern the name in the global NAME_POOL
                let interned = NAME_POOL.intern(&name);

                Ok(NameAndParent::new(interned, parent))
            }
        }

        deserializer.deserialize_tuple(2, NameAndParentVisitor)
    }
}

// ---------------------------------------------------------------------------
// SlabNode
// ---------------------------------------------------------------------------

/// A node in the filesystem index slab.
///
/// This is the primary storage type for indexed filesystem entries.
/// It's designed to be compact while still providing all necessary data.
#[derive(Debug, Serialize, Deserialize)]
pub struct SlabNode {
    /// The filename and parent reference.
    name_and_parent: NameAndParent,
    /// Child node indices (empty for files).
    pub children: ThinVec<SlabIndex>,
    /// Compact metadata.
    pub metadata: SlabNodeMetadata,
}

impl SlabNode {
    /// Creates a new SlabNode.
    ///
    /// The `name` must be a reference from a NamePool.
    pub fn new(parent: Option<SlabIndex>, name: &'static str, metadata: SlabNodeMetadata) -> Self {
        Self {
            name_and_parent: NameAndParent::new(name, OptionSlabIndex::from_option(parent)),
            children: ThinVec::new(),
            metadata,
        }
    }

    /// Returns the filename.
    #[inline]
    pub fn name(&self) -> &'static str {
        self.name_and_parent.name()
    }

    /// Returns the parent node index, if any.
    #[inline]
    pub fn parent(&self) -> Option<SlabIndex> {
        self.name_and_parent.parent()
    }

    /// Returns the file type.
    #[inline]
    pub fn file_type(&self) -> NodeFileType {
        self.metadata.file_type()
    }

    /// Returns true if this is a directory.
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.metadata.is_dir()
    }

    /// Returns true if this is a file.
    #[inline]
    pub fn is_file(&self) -> bool {
        self.metadata.is_file()
    }

    /// Adds a child node index.
    pub fn add_child(&mut self, child: SlabIndex) {
        if !self.children.contains(&child) {
            self.children.push(child);
        }
    }

    /// Removes a child node index, returns true if it was present.
    pub fn remove_child(&mut self, child: SlabIndex) -> bool {
        if let Some(pos) = self.children.iter().position(|&c| c == child) {
            self.children.remove(pos);
            true
        } else {
            false
        }
    }

    /// Returns true if this node represents a hidden file (name starts with '.').
    #[inline]
    pub fn is_hidden(&self) -> bool {
        self.name().starts_with('.')
    }

    /// Returns the file size in bytes, or None for directories.
    #[inline]
    pub fn size(&self) -> Option<u64> {
        if self.is_dir() {
            None
        } else {
            Some(self.metadata.state_type_size.raw_size())
        }
    }

    /// Returns the modification time as Unix timestamp (seconds).
    #[inline]
    pub fn modified_at(&self) -> Option<u64> {
        let mtime = self.metadata.mtime;
        if mtime == 0 {
            None
        } else {
            Some(mtime as u64)
        }
    }

    /// Returns the creation time as Unix timestamp (seconds).
    #[inline]
    pub fn created_at(&self) -> Option<u64> {
        let ctime = self.metadata.ctime;
        if ctime == 0 {
            None
        } else {
            Some(ctime as u64)
        }
    }

    /// Extracts the file extension from the name (lowercase), if any.
    #[inline]
    pub fn extension(&self) -> Option<&str> {
        let name = self.name();
        let dot_pos = name.rfind('.')?;
        if dot_pos + 1 >= name.len() || dot_pos == 0 {
            return None;
        }
        Some(&name[dot_pos + 1..])
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_type_size_encoding() {
        // Test with a regular file
        let sts = StateTypeSize::some(NodeFileType::File, 12345);
        assert_eq!(sts.file_type(), NodeFileType::File);
        assert_eq!(sts.raw_size(), 12345);
        assert!(sts.is_file());
        assert!(!sts.is_dir());

        // Test with a directory
        let sts = StateTypeSize::some(NodeFileType::Dir, 4096);
        assert_eq!(sts.file_type(), NodeFileType::Dir);
        assert_eq!(sts.raw_size(), 4096);
        assert!(sts.is_dir());
        assert!(!sts.is_file());
    }

    #[test]
    fn state_type_size_max_size() {
        let max = StateTypeSize::MAX_SIZE;
        let sts = StateTypeSize::some(NodeFileType::File, max);
        assert_eq!(sts.raw_size(), max);

        // Test overflow clamping
        let sts = StateTypeSize::some(NodeFileType::File, max + 1000);
        assert_eq!(sts.raw_size(), max);
    }

    #[test]
    fn slab_node_metadata_from_directory() {
        let metadata = SlabNodeMetadata {
            state_type_size: StateTypeSize::some(NodeFileType::Dir, 4096),
            ctime: 1700000000,
            mtime: 1700000001,
        };

        assert!(metadata.is_dir());
        assert!(!metadata.is_file());
        assert_eq!(metadata.file_type(), NodeFileType::Dir);
        assert_eq!(metadata.ctime, 1700000000);
        assert_eq!(metadata.mtime, 1700000001);
    }

    #[test]
    fn name_and_parent_basic() {
        let name: &'static str = "test.txt";
        let parent_idx = SlabIndex::new(42);

        let nap = NameAndParent::new(name, OptionSlabIndex::some(parent_idx));
        assert_eq!(nap.name(), "test.txt");
        assert_eq!(nap.parent(), Some(parent_idx));

        // Test deref
        assert_eq!(&*nap, "test.txt");
    }

    #[test]
    fn name_and_parent_no_parent() {
        let name: &'static str = "root";
        let nap = NameAndParent::new(name, OptionSlabIndex::none());
        assert_eq!(nap.parent(), None);
    }

    #[test]
    fn slab_node_basic() {
        let name: &'static str = "file.rs";
        let metadata = SlabNodeMetadata {
            state_type_size: StateTypeSize::some(NodeFileType::File, 1024),
            ctime: 1700000000,
            mtime: 1700000001,
        };

        let node = SlabNode::new(None, name, metadata);
        assert_eq!(node.name(), "file.rs");
        assert_eq!(node.parent(), None);
        assert!(node.is_file());
        assert!(!node.is_dir());
        assert!(node.children.is_empty());
    }

    #[test]
    fn slab_node_with_children() {
        let name: &'static str = "src";
        let metadata = SlabNodeMetadata {
            state_type_size: StateTypeSize::some(NodeFileType::Dir, 4096),
            ctime: 0,
            mtime: 0,
        };

        let mut node = SlabNode::new(None, name, metadata);
        let child1 = SlabIndex::new(1);
        let child2 = SlabIndex::new(2);

        node.add_child(child1);
        node.add_child(child2);
        assert_eq!(node.children.len(), 2);

        // Adding duplicate should not increase count
        node.add_child(child1);
        assert_eq!(node.children.len(), 2);

        // Remove a child
        assert!(node.remove_child(child1));
        assert_eq!(node.children.len(), 1);

        // Remove non-existent child
        assert!(!node.remove_child(child1));
    }
}
