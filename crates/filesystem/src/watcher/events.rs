//! Filesystem watching - FSEvents (macOS) and notify (cross-platform).
//!
//! Watcher callbacks send events through a crossbeam channel instead of
//! directly mutating shared index state. The index thread is the sole
//! consumer and applies changes to its owned data.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crossbeam_channel::Sender;

use super::walker::{path_in_scope, path_is_ignored};
use crate::error::{FilesystemError, Result};
use crate::indexer::RootIndexData;
use crate::storage::NAME_POOL;

#[cfg(target_os = "macos")]
use super::fsevent::{FsEvent, FsEventFlags, FsEventScanType, FsEventStream};

#[cfg(not(target_os = "macos"))]
use notify::{recommended_watcher, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// An event sent from the watcher to the index thread.
#[derive(Debug)]
pub enum WatcherEvent {
    /// Incremental path changes to apply.
    PathsChanged(Vec<PathBuf>),
    /// A full rescan is required (root was directly modified, kernel dropped events, etc.).
    RescanRequired,
    /// FSEvents history replay is complete (macOS only).
    HistoryDone,
    /// The watcher encountered an error.
    Error(String),
}

/// Creates an FSEvents watcher on macOS.
///
/// Events are sent through `event_tx` to the index thread.
#[cfg(target_os = "macos")]
pub fn create_fsevent_watcher(
    root: &PathBuf,
    ignored_roots: &[PathBuf],
    since_event_id: u64,
    event_tx: Sender<WatcherEvent>,
    last_event_id: Arc<std::sync::atomic::AtomicU64>,
) -> Result<FsEventStream> {
    let callback_root = root.clone();
    let callback_ignored = ignored_roots.to_vec();
    let stream = FsEventStream::new(
        root,
        ignored_roots,
        since_event_id,
        0.1,
        move |events| {
            process_fsevent_batch(
                &callback_root,
                &callback_ignored,
                events,
                &event_tx,
                &last_event_id,
            );
        },
    )
    .map_err(|error| {
        FilesystemError::Internal(format!(
            "failed to start FSEvents watcher for {}: {error}",
            root.display()
        ))
    })?;
    Ok(stream)
}

/// Processes a batch of FSEvents and sends them to the index thread.
#[cfg(target_os = "macos")]
fn process_fsevent_batch(
    root: &PathBuf,
    _ignored_roots: &[PathBuf],
    events: Vec<FsEvent>,
    event_tx: &Sender<WatcherEvent>,
    last_event_id: &std::sync::atomic::AtomicU64,
) {
    if events.is_empty() {
        return;
    }

    let saw_history_done = events
        .iter()
        .any(|event| event.flags.contains(FsEventFlags::HISTORY_DONE));

    // Update last_event_id to the maximum in this batch
    let max_event_id = events.iter().map(|e| e.event_id).max().unwrap_or(0);
    if max_event_id > 0 {
        last_event_id.fetch_max(max_event_id, Ordering::Relaxed);
    }

    if saw_history_done {
        let _ = event_tx.send(WatcherEvent::HistoryDone);
    }

    // Check if any event requires a full rescan
    let needs_rescan = events.iter().any(|event| {
        event.scan_type == FsEventScanType::ReScan
            || (matches!(
                event.scan_type,
                FsEventScanType::SingleNode | FsEventScanType::Folder
            ) && event.path == *root)
    });

    if needs_rescan {
        let _ = event_tx.send(WatcherEvent::RescanRequired);
        return;
    }

    // Collect non-Nop paths and send
    let paths: Vec<PathBuf> = events
        .into_iter()
        .filter(|e| e.scan_type != FsEventScanType::Nop)
        .map(|e| e.path)
        .collect();

    if !paths.is_empty() {
        let _ = event_tx.send(WatcherEvent::PathsChanged(paths));
    }
}

/// Creates a notify watcher on non-macOS platforms.
///
/// Events are sent through `event_tx` to the index thread.
#[cfg(not(target_os = "macos"))]
pub fn create_index_watcher(
    root: PathBuf,
    root_is_dir: bool,
    event_tx: Sender<WatcherEvent>,
) -> Result<RecommendedWatcher> {
    use std::path::Path;

    let mut watcher =
        recommended_watcher(move |event_result: notify::Result<Event>| match event_result {
            Ok(event) => {
                if matches!(event.kind, EventKind::Access(_)) {
                    return;
                }
                if event.paths.is_empty() {
                    let _ = event_tx.send(WatcherEvent::RescanRequired);
                } else {
                    let _ = event_tx.send(WatcherEvent::PathsChanged(event.paths));
                }
            }
            Err(error) => {
                let _ = event_tx.send(WatcherEvent::Error(error.to_string()));
            }
        })
        .map_err(|error| {
            FilesystemError::Internal(format!(
                "failed to create filesystem watcher for {}: {error}",
                root.display()
            ))
        })?;

    let (watch_target, recursive_mode) = if root_is_dir {
        (root.clone(), RecursiveMode::Recursive)
    } else {
        (
            root.parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| root.clone()),
            RecursiveMode::NonRecursive,
        )
    };

    watcher
        .watch(&watch_target, recursive_mode)
        .map_err(|error| {
            FilesystemError::Internal(format!(
                "failed to watch {}: {error}",
                watch_target.display()
            ))
        })?;

    Ok(watcher)
}

/// Applies a single path change to the index.
///
/// Called by the index thread on its owned data — no locks needed.
pub fn apply_path_change(
    root: &PathBuf,
    root_is_dir: bool,
    ignored_roots: &[PathBuf],
    data: &mut RootIndexData,
    changed_path: &std::path::Path,
) {
    if !path_in_scope(root, root_is_dir, changed_path) {
        return;
    }

    if path_is_ignored(ignored_roots, changed_path) {
        remove_path_and_descendants(data, changed_path);
        return;
    }

    if changed_path == root.as_path() {
        // Root itself changed — caller should handle rescan
        return;
    }

    if changed_path.exists() {
        // Remove stale entry (and descendants if it was a directory that changed type).
        remove_path_and_descendants(data, changed_path);
        // Upsert just this single node. With kFSEventStreamCreateFlagFileEvents,
        // FSEvents reports every individual file — children arrive as their own
        // events, so recursive directory walks are unnecessary (and catastrophically
        // slow for large directories like node_modules or .cargo).
        data.upsert_entry(changed_path, &NAME_POOL);
    } else {
        remove_path_and_descendants(data, changed_path);
    }
}

/// Removes a path and all its descendants from the index.
fn remove_path_and_descendants(data: &mut RootIndexData, target: &std::path::Path) {
    // Use tree-based path lookup and remove
    data.remove_entry(target);
}

