//! Filesystem indexing module.
//!
//! This module handles building and maintaining the filesystem index:
//! - Walking the filesystem to build node trees
//! - Constructing slab-based index data structures
//! - Persisting indexes to disk cache

mod build;
mod construct;
mod data;
mod file_nodes;
mod node_view;
mod persistence;
mod shared;
mod walk;

// Re-export main types
pub use build::{
    unix_now_secs, FlushDecision, FlushSignal, FlushWorkerHandle, IndexBuildProgress,
    IndexBuildState, ProgressSnapshot,
};
pub use construct::NameIndex;
pub use data::RootIndexData;
pub use file_nodes::FileNodes;
pub use node_view::NodeView;
pub use persistence::{
    load_index_snapshot, write_index_snapshot, PersistentStorage, RootIndexKey,
    INDEX_CACHE_MAX_AGE_SECS, INDEX_CACHE_VERSION,
};
pub use shared::SharedRootIndex;
pub use walk::{walk_it, Node, WalkData};
