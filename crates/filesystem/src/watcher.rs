//! Filesystem watching module.
//!
//! This module handles real-time filesystem monitoring:
//! - FSEvents on macOS for efficient event streaming
//! - notify on other platforms
//! - Event coalescing and path change handling
//! - macOS Finder tags support

mod events;
mod file_tags;
mod walker;

#[cfg(target_os = "macos")]
mod fsevent;

// Re-export event handling
pub use events::{apply_path_change, mark_index_dirty};

#[cfg(target_os = "macos")]
pub use events::create_fsevent_watcher;

#[cfg(not(target_os = "macos"))]
pub use events::create_index_watcher;

// Re-export walker utilities
pub use walker::{coalesce_event_paths, path_in_scope, path_is_ignored};

// Re-export FSEvents (macOS only)
#[cfg(target_os = "macos")]
pub use fsevent::{FsEvent, FsEventScanType, FsEventStream};

// Re-export file tags
pub use file_tags::{
    file_has_any_tag, read_tags_from_path, search_tags_using_mdfind as search_tags_mdfind,
};
