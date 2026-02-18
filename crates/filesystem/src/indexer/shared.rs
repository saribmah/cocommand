//! Root index configuration (immutable after creation).

use std::path::PathBuf;

/// Immutable configuration for a root index.
///
/// Unlike the old `SharedRootIndex`, this struct contains no mutable state.
/// All mutable state (index data, pending changes, flush timing) is owned
/// exclusively by the index thread.
#[derive(Debug, Clone)]
pub struct RootIndexConfig {
    /// Root path being indexed.
    pub root: PathBuf,
    /// Whether the root is a directory.
    pub root_is_dir: bool,
    /// Paths to ignore during indexing.
    pub ignored_roots: Vec<PathBuf>,
    /// Path to the cache file.
    pub cache_path: PathBuf,
}
