use std::cmp::Ordering;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EntryKindFilter {
    All,
    File,
    Directory,
}

impl EntryKindFilter {
    pub(super) fn parse(raw: Option<&str>) -> CoreResult<Self> {
        match raw.unwrap_or("all") {
            "all" => Ok(Self::All),
            "file" => Ok(Self::File),
            "directory" => Ok(Self::Directory),
            other => Err(CoreError::InvalidInput(format!(
                "unsupported kind: {other} (expected one of: all, file, directory)"
            ))),
        }
    }

    pub(super) fn matches(self, file_type: &fs::FileType) -> bool {
        match self {
            Self::All => true,
            Self::File => file_type.is_file(),
            Self::Directory => file_type.is_dir(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ListSortKey {
    Name,
    Path,
    Modified,
    Size,
    Type,
}

impl ListSortKey {
    pub(super) fn parse(raw: Option<&str>) -> CoreResult<Self> {
        match raw.unwrap_or("name") {
            "name" => Ok(Self::Name),
            "path" => Ok(Self::Path),
            "modified" => Ok(Self::Modified),
            "size" => Ok(Self::Size),
            "type" => Ok(Self::Type),
            other => Err(CoreError::InvalidInput(format!(
                "unsupported sortBy: {other} (expected one of: name, path, modified, size, type)"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SortOrder {
    Asc,
    Desc,
}

impl SortOrder {
    pub(super) fn parse(raw: Option<&str>) -> CoreResult<Self> {
        match raw.unwrap_or("asc") {
            "asc" => Ok(Self::Asc),
            "desc" => Ok(Self::Desc),
            other => Err(CoreError::InvalidInput(format!(
                "unsupported sortOrder: {other} (expected one of: asc, desc)"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FileSystemEntry {
    pub(super) path: String,
    pub(super) name: String,
    #[serde(rename = "type")]
    pub(super) entry_type: String,
    pub(super) size: Option<u64>,
    pub(super) modified_at: Option<u64>,
    /// Base64-encoded PNG icon data URI (e.g., "data:image/png;base64,...").
    /// Only populated when icons are requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) icon: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SearchPayload {
    pub(super) query: String,
    pub(super) root: String,
    pub(super) results: Vec<FileSystemEntry>,
    pub(super) count: usize,
    pub(super) truncated: bool,
    pub(super) scanned: usize,
    pub(super) errors: usize,
    pub(super) index_state: String,
    pub(super) index_scanned_files: usize,
    pub(super) index_scanned_dirs: usize,
    pub(super) index_started_at: Option<u64>,
    pub(super) index_last_update_at: Option<u64>,
    pub(super) index_finished_at: Option<u64>,
    /// Terms that should be highlighted in search results.
    /// Sorted, deduplicated, and lowercased for case-insensitive matching.
    pub(super) highlight_terms: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct IndexStatusPayload {
    pub(super) state: String,
    pub(super) root: String,
    pub(super) ignored_paths: Vec<String>,
    pub(super) indexed_entries: usize,
    pub(super) scanned_files: usize,
    pub(super) scanned_dirs: usize,
    pub(super) started_at: Option<u64>,
    pub(super) last_update_at: Option<u64>,
    pub(super) finished_at: Option<u64>,
    pub(super) errors: usize,
    pub(super) watcher_enabled: bool,
    pub(super) cache_path: String,
    /// Count of full rescans performed. Incremented when FS events trigger a full rescan.
    /// UI can use this to detect when search results may be stale and need refresh.
    pub(super) rescan_count: u64,
    /// Last error message if state is "error".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) last_error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ListPayload {
    pub(super) path: String,
    pub(super) recursive: bool,
    pub(super) results: Vec<FileSystemEntry>,
    pub(super) count: usize,
    pub(super) truncated: bool,
    pub(super) errors: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ReadFilePayload {
    pub(super) path: String,
    pub(super) content: String,
    pub(super) offset: u64,
    pub(super) bytes_read: usize,
    pub(super) total_bytes: u64,
    pub(super) truncated: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PathInfoPayload {
    pub(super) path: String,
    pub(super) exists: bool,
    pub(super) name: Option<String>,
    pub(super) parent: Option<String>,
    pub(super) extension: Option<String>,
    #[serde(rename = "type")]
    pub(super) entry_type: Option<String>,
    pub(super) size: Option<u64>,
    pub(super) modified_at: Option<u64>,
    pub(super) readonly: Option<bool>,
    pub(super) hidden: Option<bool>,
}

/// Compact entry type enum instead of string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum EntryType {
    File,
    Directory,
    Symlink,
    Other,
}

impl EntryType {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "directory",
            Self::Symlink => "symlink",
            Self::Other => "other",
        }
    }

    pub(super) fn from_file_type(file_type: &fs::FileType) -> Self {
        if file_type.is_dir() {
            Self::Directory
        } else if file_type.is_file() {
            Self::File
        } else if file_type.is_symlink() {
            Self::Symlink
        } else {
            Self::Other
        }
    }
}

pub(super) fn build_entry(path: &Path, metadata: &fs::Metadata) -> FileSystemEntry {
    let file_type = metadata.file_type();
    FileSystemEntry {
        path: path.to_string_lossy().to_string(),
        name: path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string()),
        entry_type: EntryType::from_file_type(&file_type).as_str().to_string(),
        size: file_type.is_file().then_some(metadata.len()),
        modified_at: modified_secs(metadata),
        icon: None,
    }
}

pub(super) fn sort_entries(entries: &mut [FileSystemEntry], key: ListSortKey, order: SortOrder) {
    entries.sort_by(|left, right| {
        let ordering = match key {
            ListSortKey::Name => left.name.cmp(&right.name),
            ListSortKey::Path => left.path.cmp(&right.path),
            ListSortKey::Modified => left.modified_at.cmp(&right.modified_at),
            ListSortKey::Size => left.size.cmp(&right.size),
            ListSortKey::Type => left.entry_type.cmp(&right.entry_type),
        };
        match order {
            SortOrder::Asc => ordering,
            SortOrder::Desc => reverse_ordering(ordering),
        }
    });
}

fn reverse_ordering(ordering: Ordering) -> Ordering {
    match ordering {
        Ordering::Less => Ordering::Greater,
        Ordering::Greater => Ordering::Less,
        Ordering::Equal => Ordering::Equal,
    }
}

pub(super) fn modified_secs(metadata: &fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_secs())
}

pub(super) fn is_hidden_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.starts_with('.'))
        .unwrap_or(false)
}

// Conversion from filesystem crate types to API types

impl From<filesystem::FileEntry> for FileSystemEntry {
    fn from(entry: filesystem::FileEntry) -> Self {
        Self {
            path: entry.path,
            name: entry.name,
            entry_type: entry.file_type.as_str().to_string(),
            size: entry.size,
            modified_at: entry.modified_at,
            icon: None,
        }
    }
}

impl From<filesystem::SearchResult> for SearchPayload {
    fn from(result: filesystem::SearchResult) -> Self {
        Self {
            query: result.query,
            root: result.root,
            results: result.entries.into_iter().map(Into::into).collect(),
            count: result.count,
            truncated: result.truncated,
            scanned: result.scanned,
            errors: result.errors,
            index_state: result.index_state,
            index_scanned_files: result.index_scanned_files,
            index_scanned_dirs: result.index_scanned_dirs,
            index_started_at: result.index_started_at,
            index_last_update_at: result.index_last_update_at,
            index_finished_at: result.index_finished_at,
            highlight_terms: result.highlight_terms,
        }
    }
}

impl From<filesystem::IndexStatus> for IndexStatusPayload {
    fn from(status: filesystem::IndexStatus) -> Self {
        Self {
            state: status.state,
            root: status.root,
            ignored_paths: status.ignored_paths,
            indexed_entries: status.indexed_entries,
            scanned_files: status.scanned_files,
            scanned_dirs: status.scanned_dirs,
            started_at: status.started_at,
            last_update_at: status.last_update_at,
            finished_at: status.finished_at,
            errors: status.errors,
            watcher_enabled: status.watcher_enabled,
            cache_path: status.cache_path,
            rescan_count: status.rescan_count,
            last_error: status.last_error,
        }
    }
}

impl From<filesystem::KindFilter> for EntryKindFilter {
    fn from(filter: filesystem::KindFilter) -> Self {
        match filter {
            filesystem::KindFilter::All => EntryKindFilter::All,
            filesystem::KindFilter::File => EntryKindFilter::File,
            filesystem::KindFilter::Directory => EntryKindFilter::Directory,
        }
    }
}

impl From<EntryKindFilter> for filesystem::KindFilter {
    fn from(filter: EntryKindFilter) -> Self {
        match filter {
            EntryKindFilter::All => filesystem::KindFilter::All,
            EntryKindFilter::File => filesystem::KindFilter::File,
            EntryKindFilter::Directory => filesystem::KindFilter::Directory,
        }
    }
}
