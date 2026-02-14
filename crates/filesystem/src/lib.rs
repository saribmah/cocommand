//! Filesystem indexing, search, and query library.
//!
//! This crate provides core filesystem functionality:
//! - Memory-mapped slab storage for file metadata
//! - Query parsing and search evaluation
//! - Persistent cache with compression
//! - Filesystem indexing with watch support

pub mod cancel;
pub mod error;
pub mod file_tags;
pub mod index;
pub mod namepool;
pub mod query;
pub mod slab;
pub mod types;

#[cfg(target_os = "macos")]
pub mod fsevent;

// Re-export main types
pub use cancel::CancellationToken;
pub use error::{FilesystemError, Result};
pub use index::{FileSystemIndexManager, RootIndexKey};
pub use namepool::{NamePool, NAME_POOL};
pub use query::{QueryExpression, QueryParser, SearchQueryMatcher};
pub use types::{FileEntry, FileType, IndexStatus, KindFilter, SearchResult};
