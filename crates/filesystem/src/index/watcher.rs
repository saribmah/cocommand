//! Filesystem watching - FSEvents (macOS) and notify (cross-platform).

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::build::IndexBuildState;
use super::data::RootIndexData;
use super::shared::SharedRootIndex;
use super::walker::{coalesce_event_paths, path_in_scope, path_is_ignored};
use crate::error::Result;
use crate::namepool::NAME_POOL;

#[cfg(target_os = "macos")]
use crate::fsevent::{FsEvent, FsEventScanType, FsEventStream};

#[cfg(not(target_os = "macos"))]
use notify::{recommended_watcher, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// Creates an FSEvents watcher on macOS.
#[cfg(target_os = "macos")]
pub fn create_fsevent_watcher(
    shared: Arc<SharedRootIndex>,
    since_event_id: u64,
) -> Result<FsEventStream> {
    let callback_shared = shared.clone();
    let stream = FsEventStream::new(
        &shared.root,
        &shared.ignored_roots,
        since_event_id,
        0.05,
        move |events| {
            apply_fsevent_batch(callback_shared.as_ref(), events);
        },
    );
    Ok(stream)
}

/// Applies a batch of FSEvents.
#[cfg(target_os = "macos")]
fn apply_fsevent_batch(shared: &SharedRootIndex, events: Vec<FsEvent>) {
    if events.is_empty() {
        return;
    }

    // Update last_event_id to the maximum in this batch
    let max_event_id = events.iter().map(|e| e.event_id).max().unwrap_or(0);
    if max_event_id > 0 {
        shared
            .last_event_id
            .fetch_max(max_event_id, Ordering::Relaxed);
    }

    // If we're still building the index, queue paths for later
    if IndexBuildState::load(&shared.build_state) == IndexBuildState::Building {
        let paths: Vec<PathBuf> = events
            .into_iter()
            .filter(|e| e.scan_type != FsEventScanType::Nop)
            .map(|e| e.path)
            .collect();
        if !paths.is_empty() {
            enqueue_pending_paths(shared, paths);
        }
        return;
    }

    // Check if any event requires a full rescan
    let needs_rescan = events
        .iter()
        .any(|e| e.scan_type == FsEventScanType::ReScan);

    if needs_rescan {
        // Increment rescan count to signal UI that results may be stale (Cardinal approach)
        shared.increment_rescan_count();
        let mut data = match shared.data.write() {
            Ok(data) => data,
            Err(_) => return,
        };
        match build_snapshot_for_rescan(shared) {
            Ok(snapshot) => *data = snapshot,
            Err(_) => data.errors += 1,
        }
        drop(data);
        mark_index_dirty(shared);
        return;
    }

    // Collect non-Nop paths, coalesce, and apply incrementally
    let paths: Vec<PathBuf> = events
        .into_iter()
        .filter(|e| e.scan_type != FsEventScanType::Nop)
        .map(|e| e.path)
        .collect();

    if paths.is_empty() {
        return;
    }

    let mut data = match shared.data.write() {
        Ok(data) => data,
        Err(_) => return,
    };

    for changed_path in coalesce_event_paths(paths) {
        apply_path_change(shared, &mut data, &changed_path);
    }
    drop(data);
    mark_index_dirty(shared);
}

/// Creates a notify watcher on non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn create_index_watcher(shared: Arc<SharedRootIndex>) -> Result<RecommendedWatcher> {
    use crate::error::FilesystemError;
    use std::path::Path;

    let callback_shared = shared.clone();
    let mut watcher =
        recommended_watcher(
            move |event_result: notify::Result<Event>| match event_result {
                Ok(event) => apply_notify_event(callback_shared.as_ref(), event),
                Err(_) => {
                    if let Ok(mut data) = callback_shared.data.write() {
                        data.errors += 1;
                    }
                }
            },
        )
        .map_err(|error| {
            FilesystemError::Internal(format!(
                "failed to create filesystem watcher for {}: {error}",
                shared.root.display()
            ))
        })?;

    let (watch_target, recursive_mode) = if shared.root_is_dir {
        (shared.root.clone(), RecursiveMode::Recursive)
    } else {
        (
            shared
                .root
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| shared.root.clone()),
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

/// Applies a notify event on non-macOS platforms.
#[cfg(not(target_os = "macos"))]
fn apply_notify_event(shared: &SharedRootIndex, event: Event) {
    if matches!(event.kind, EventKind::Access(_)) {
        return;
    }

    if IndexBuildState::load(&shared.build_state) == IndexBuildState::Building {
        enqueue_pending_paths(shared, event.paths);
        return;
    }

    let mut data = match shared.data.write() {
        Ok(data) => data,
        Err(_) => return,
    };

    if event.paths.is_empty() {
        match build_snapshot_for_rescan(shared) {
            Ok(snapshot) => *data = snapshot,
            Err(_) => data.errors += 1,
        }
        drop(data);
        mark_index_dirty(shared);
        return;
    }

    for changed_path in coalesce_event_paths(event.paths) {
        apply_path_change(shared, &mut data, &changed_path);
    }
    drop(data);
    mark_index_dirty(shared);
}

/// Enqueues pending paths for processing after build completes.
pub fn enqueue_pending_paths(shared: &SharedRootIndex, mut paths: Vec<PathBuf>) {
    if paths.is_empty() {
        paths.push(shared.root.clone());
    }
    if let Ok(mut pending) = shared.pending_changes.lock() {
        pending.extend(paths);
        if pending.len() > 4096 {
            let drained = std::mem::take(&mut *pending);
            *pending = coalesce_event_paths(drained);
        }
    }
}

/// Applies a single path change to the index.
pub fn apply_path_change(
    shared: &SharedRootIndex,
    data: &mut RootIndexData,
    changed_path: &std::path::Path,
) {
    if !path_in_scope(&shared.root, shared.root_is_dir, changed_path) {
        return;
    }

    if path_is_ignored(&shared.ignored_roots, changed_path) {
        remove_path_and_descendants(data, changed_path);
        return;
    }

    if changed_path == shared.root {
        match build_snapshot_for_rescan(shared) {
            Ok(snapshot) => *data = snapshot,
            Err(_) => data.errors += 1,
        }
        return;
    }

    if changed_path.exists() {
        remove_path_and_descendants(data, changed_path);
        // Recursively upsert the path and its descendants
        upsert_path_recursive(data, changed_path, &shared.ignored_roots);
    } else {
        remove_path_and_descendants(data, changed_path);
    }
}

/// Recursively upserts a path and its descendants into the index.
fn upsert_path_recursive(data: &mut RootIndexData, path: &std::path::Path, ignored: &[PathBuf]) {
    // Check if path should be ignored
    if ignored.iter().any(|ig| path == ig || path.starts_with(ig)) {
        return;
    }

    // Upsert this path
    data.upsert_entry(path, &NAME_POOL);

    // If it's a directory, recurse into children
    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            // Collect and sort entries by name for deterministic order
            let mut children: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
            children.sort();

            for child_path in children {
                upsert_path_recursive(data, &child_path, ignored);
            }
        }
    }
}

/// Removes a path and all its descendants from the index.
fn remove_path_and_descendants(data: &mut RootIndexData, target: &std::path::Path) {
    // Use tree-based path lookup and remove
    data.remove_entry(target);
}

/// Builds a fresh snapshot for rescan.
fn build_snapshot_for_rescan(shared: &SharedRootIndex) -> crate::error::Result<RootIndexData> {
    use super::fswalk::WalkData as FsWalkData;

    // Use the new Cardinal-style two-phase approach
    let walk_data = FsWalkData::new(&shared.root, &shared.ignored_roots);

    // Build using from_walk which does two-phase: walk -> tree -> slab
    match RootIndexData::from_walk(&walk_data) {
        Some(snapshot) => Ok(snapshot),
        None => {
            // Walk was cancelled (shouldn't happen with no cancel flag)
            Ok(RootIndexData::new())
        }
    }
}

/// Marks the index as dirty (needs flushing).
pub fn mark_index_dirty(shared: &SharedRootIndex) {
    shared.flush_signal.mark_dirty();
}
