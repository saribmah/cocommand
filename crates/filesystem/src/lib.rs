//! Filesystem indexing, search, and query library.
//!
//! This crate provides core filesystem functionality:
//! - Memory-mapped slab storage for file metadata
//! - Query parsing and search evaluation
//! - Persistent cache with compression
//! - Filesystem indexing with watch support
//!
//! ## Architecture
//!
//! Each indexed root gets its own **index thread** that exclusively owns
//! the `RootIndexData`. All communication (search, status, watcher events)
//! happens through crossbeam channels — no shared mutable state, no locks
//! on the hot path, no deadlocks.
//!
//! ## Module Structure
//!
//! - `storage` - Low-level storage primitives (slab, namepool)
//! - `indexer` - Index building and data structures
//! - `search` - Search engine and manager API
//! - `watcher` - Filesystem watching (FSEvents on macOS, notify on others)
//! - `query` - Query parsing and evaluation

pub mod cancel;
pub mod error;
pub mod indexer;
pub mod query;
pub mod search;
pub mod storage;
pub mod types;
pub mod watcher;

// Re-export main types
pub use cancel::{CancellationToken, SearchVersionTracker};
pub use error::{FilesystemError, Result};
pub use indexer::RootIndexKey;
pub use query::{QueryExpression, QueryParser, SearchQueryMatcher};
pub use search::FileSystemIndexManager;
pub use storage::{NamePool, NAME_POOL};
pub use types::{FileEntry, FileType, IndexStatus, KindFilter, SearchResult};
