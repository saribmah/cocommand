//! Storage layer for filesystem indexing.
//!
//! This module provides the low-level data storage primitives:
//! - Memory-mapped slab allocator for efficient large-scale storage
//! - Name interning pool for deduplicating filenames

mod entry;
mod index_types;
mod namepool;
mod node;
mod slab;
mod thin;

// Re-export slab types
pub use index_types::{OptionSlabIndex, SlabIndex, SortedSlabIndices};
pub use node::{NodeFileType, SlabNode, SlabNodeMetadata, StateTypeSize};
pub use slab::Slab;
pub use thin::ThinSlab;

// Re-export namepool
pub use namepool::{NamePool, NAME_POOL};
