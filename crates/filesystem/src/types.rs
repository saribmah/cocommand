//! Internal types for filesystem indexing results.
//!
//! These are the core result types used internally. The cocommand crate
//! converts these to API payload types for serialization.

use serde::{Deserialize, Serialize};

/// File type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    File,
    Directory,
    Symlink,
    Other,
}

impl FileType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "directory",
            Self::Symlink => "symlink",
            Self::Other => "other",
        }
    }
}

/// A file entry in search results.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub file_type: FileType,
    pub size: Option<u64>,
    pub modified_at: Option<u64>,
}

/// Kind filter for file entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KindFilter {
    #[default]
    All,
    File,
    Directory,
}

impl KindFilter {
    pub fn matches(&self, file_type: FileType) -> bool {
        match self {
            Self::All => true,
            Self::File => file_type == FileType::File,
            Self::Directory => file_type == FileType::Directory,
        }
    }
}

/// Search results from the filesystem index.
#[derive(Debug)]
pub struct SearchResult {
    /// The query that was executed.
    pub query: String,
    /// The root path that was searched.
    pub root: String,
    /// Matching file entries.
    pub entries: Vec<FileEntry>,
    /// Total count of matches (may differ from entries.len() if truncated).
    pub count: usize,
    /// Whether results were truncated due to max_results limit.
    pub truncated: bool,
    /// Number of entries scanned during search.
    pub scanned: usize,
    /// Number of errors encountered.
    pub errors: usize,
    /// Index state at time of search.
    pub index_state: String,
    /// Number of files scanned during indexing.
    pub index_scanned_files: usize,
    /// Number of directories scanned during indexing.
    pub index_scanned_dirs: usize,
    /// Unix timestamp when indexing started.
    pub index_started_at: Option<u64>,
    /// Unix timestamp of last index update.
    pub index_last_update_at: Option<u64>,
    /// Unix timestamp when indexing finished.
    pub index_finished_at: Option<u64>,
    /// Terms to highlight in search results.
    pub highlight_terms: Vec<String>,
}

/// Index status information.
#[derive(Debug)]
pub struct IndexStatus {
    /// Current state of the index.
    pub state: String,
    /// Root path being indexed.
    pub root: String,
    /// Paths excluded from indexing.
    pub ignored_paths: Vec<String>,
    /// Number of entries in the index.
    pub indexed_entries: usize,
    /// Number of files scanned.
    pub scanned_files: usize,
    /// Number of directories scanned.
    pub scanned_dirs: usize,
    /// Unix timestamp when indexing started.
    pub started_at: Option<u64>,
    /// Unix timestamp of last update.
    pub last_update_at: Option<u64>,
    /// Unix timestamp when indexing finished.
    pub finished_at: Option<u64>,
    /// Number of errors encountered.
    pub errors: usize,
    /// Whether the filesystem watcher is enabled.
    pub watcher_enabled: bool,
    /// Path to the index cache file.
    pub cache_path: String,
    /// Count of full rescans performed.
    pub rescan_count: u64,
    /// Last error message if state is "error".
    pub last_error: Option<String>,
}
