//! Filesystem indexing module.
//!
//! This module provides fast filesystem indexing and search capabilities using
//! memory-mapped slab storage for efficient handling of large directory trees.
//!
//! ## Architecture (Cardinal-style)
//!
//! The indexing follows Cardinal's two-phase approach:
//! 1. **Walk phase** (`fswalk`): Builds a `Node` tree with children sorted by name
//! 2. **Convert phase** (`construct`): Transforms tree to slab + name_index
//!
//! This enables efficient O(1) ordered insertion during construction because
//! the preorder traversal visits nodes in lexicographic path order.
//!
//! ## Module Structure
//!
//! - `build` - Build state and progress tracking
//! - `construct` - Slab and name index construction from Node tree
//! - `data` - Core index data structures (FileNodes + NameIndex)
//! - `file_nodes` - Hierarchical tree wrapper for slab storage
//! - `fswalk` - Parallel filesystem walking that builds Node tree
//! - `manager` - Main API (FileSystemIndexManager)
//! - `node_view` - Helpers for computing derived node properties
//! - `persistence` - Cache read/write operations
//! - `search` - Query evaluation
//! - `shared` - Shared state for root indexes
//! - `walker` - Legacy walker (coalesce_event_paths for watcher)
//! - `watcher` - FSEvents (macOS) / notify (other platforms) integration

mod build;
mod construct;
mod data;
mod file_nodes;
mod fswalk;
mod manager;
mod node_view;
mod persistence;
mod search;
mod shared;
mod walker;
mod watcher;

// Re-export main types
pub use manager::FileSystemIndexManager;
pub use persistence::RootIndexKey;
