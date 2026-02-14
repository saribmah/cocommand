//! Memory-mapped slab allocator for filesystem index nodes.
//!
//! This module provides a disk-backed slab allocator that stores entries in a
//! memory-mapped temporary file. This allows the OS to page large indexes in
//! and out of memory, enabling efficient handling of millions of filesystem
//! entries without exhausting heap memory.
//!
//! The design mirrors Cardinal's `slab-mmap` crate for compatibility.
//!
//! ## Module Structure
//!
//! - `index_types` - Compact index types (`SlabIndex`, `OptionSlabIndex`)
//! - `entry` - Internal entry enum for slab slots
//! - `mmap` - Memory-mapped slab allocator (`Slab<T>`)
//! - `thin` - High-level wrapper (`ThinSlab<T>`)
//! - `node` - Filesystem node types (`SlabNode`, `SlabNodeMetadata`, etc.)

mod entry;
mod index_types;
mod mmap;
mod node;
mod thin;

// Re-export public types
pub use index_types::{OptionSlabIndex, SlabIndex, SortedSlabIndices};
pub use node::{NodeFileType, SlabNode, SlabNodeMetadata};
pub use thin::ThinSlab;

// Test-only re-exports
#[cfg(test)]
pub use node::StateTypeSize;
