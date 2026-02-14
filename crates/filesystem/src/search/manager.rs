//! FileSystemIndexManager - main API for filesystem indexing.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, RwLock, TryLockError};
use std::thread;
use std::time::Instant;

use super::engine::search_index_data;
use crate::cancel::SearchVersionTracker;
use crate::error::{canonicalize_existing_path, lock_poisoned_error, FilesystemError, Result};
use crate::indexer::{
    load_index_snapshot, unix_now_secs, write_index_snapshot, FlushDecision, FlushSignal,
    FlushWorkerHandle, IndexBuildProgress, IndexBuildState, RootIndexData, RootIndexKey,
    SharedRootIndex, WalkData,
};
use crate::types::{IndexStatus, KindFilter, SearchResult};
use crate::watcher::{apply_path_change, coalesce_event_paths, mark_index_dirty};

#[cfg(target_os = "macos")]
use crate::watcher::{create_fsevent_watcher, FsEventStream};
#[cfg(target_os = "macos")]
type WatcherHandle = FsEventStream;

#[cfg(not(target_os = "macos"))]
use crate::watcher::create_index_watcher;
#[cfg(not(target_os = "macos"))]
use notify::RecommendedWatcher;
#[cfg(not(target_os = "macos"))]
type WatcherHandle = RecommendedWatcher;

/// A root index with its watcher and flush worker.
struct RootIndex {
    shared: Arc<SharedRootIndex>,
    first_status_timing_logged: AtomicBool,
    #[cfg(target_os = "macos")]
    _watcher: Arc<Mutex<Option<FsEventStream>>>,
    #[cfg(not(target_os = "macos"))]
    _watcher: Arc<Mutex<Option<RecommendedWatcher>>>,
    _flush_worker: FlushWorkerHandle,
}

impl std::fmt::Debug for RootIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RootIndex")
            .field("shared", &self.shared)
            .field("_watcher", &"<watcher>")
            .finish()
    }
}

impl RootIndex {
    fn new(key: RootIndexKey, cache_dir: PathBuf) -> Result<Self> {
        let init_started = Instant::now();
        let root_metadata = fs::symlink_metadata(&key.root).map_err(|error| {
            FilesystemError::InvalidInput(format!(
                "unable to access root path {}: {error}",
                key.root.display()
            ))
        })?;
        let root_is_dir = root_metadata.file_type().is_dir();
        let cache_path = key.cache_path(&cache_dir);
        let flush_signal = Arc::new(FlushSignal::default());

        let cache_load_started = Instant::now();
        let loaded_snapshot =
            load_index_snapshot(&cache_path, &key.root, root_is_dir, &key.ignored_roots);
        let cache_load_ms = cache_load_started.elapsed().as_millis();

        let (snapshot, build_state, build_progress, cached_event_id, cache_loaded) =
            match loaded_snapshot {
                Some((snapshot, saved_at, event_id)) => (
                    snapshot,
                    IndexBuildState::Ready,
                    IndexBuildProgress::new_ready(saved_at),
                    event_id,
                    true,
                ),
                None => (
                    RootIndexData::default(),
                    IndexBuildState::Idle,
                    IndexBuildProgress::default(),
                    0u64,
                    false,
                ),
            };
        let cached_entries = snapshot.len();

        #[cfg(target_os = "macos")]
        let since_event_id = if cached_event_id > 0 {
            cached_event_id
        } else {
            FsEventStream::current_event_id()
        };
        #[cfg(not(target_os = "macos"))]
        let _ = cached_event_id;

        let shared = Arc::new(SharedRootIndex {
            root: key.root.clone(),
            root_is_dir,
            ignored_roots: key.ignored_roots.clone(),
            cache_path: cache_path.clone(),
            flush_signal,
            build_state: AtomicU8::new(build_state as u8),
            build_progress,
            build_generation: AtomicU64::new(0),
            build_cancel: Mutex::new(None),
            build_last_error: Mutex::new(None),
            pending_changes: Mutex::new(Vec::new()),
            data: RwLock::new(snapshot),
            #[cfg(target_os = "macos")]
            last_event_id: AtomicU64::new(since_event_id),
            #[cfg(not(target_os = "macos"))]
            last_event_id: AtomicU64::new(0),
            rescan_count: AtomicU64::new(0),
        });

        let flush_worker = spawn_flush_worker(shared.clone());
        let watcher_slot: Arc<Mutex<Option<WatcherHandle>>> = Arc::new(Mutex::new(None));

        let watcher_shared = shared.clone();
        let watcher_slot_clone = watcher_slot.clone();
        thread::spawn(move || {
            #[cfg(target_os = "macos")]
            let watcher = match create_fsevent_watcher(watcher_shared.clone(), since_event_id) {
                Ok(watcher) => Some(watcher),
                Err(error) => {
                    if let Ok(mut data) = watcher_shared.data.write() {
                        data.errors += 1;
                    }
                    log::warn!(
                        "filesystem watcher disabled for {}: {}",
                        watcher_shared.root.display(),
                        error
                    );
                    None
                }
            };

            #[cfg(not(target_os = "macos"))]
            let watcher = match create_index_watcher(watcher_shared.clone()) {
                Ok(watcher) => Some(watcher),
                Err(error) => {
                    if let Ok(mut data) = watcher_shared.data.write() {
                        data.errors += 1;
                    }
                    log::warn!(
                        "filesystem watcher disabled for {}: {}",
                        watcher_shared.root.display(),
                        error
                    );
                    None
                }
            };

            if let Ok(mut slot) = watcher_slot_clone.lock() {
                *slot = watcher;
            }
        });

        log::info!(
            "filesystem index init root={} cache_loaded={} cache_load_ms={} cached_entries={} state={} total_init_ms={}",
            key.root.display(),
            cache_loaded,
            cache_load_ms,
            cached_entries,
            build_state.as_str(),
            init_started.elapsed().as_millis(),
        );

        Ok(Self {
            shared,
            first_status_timing_logged: AtomicBool::new(false),
            _watcher: watcher_slot,
            _flush_worker: flush_worker,
        })
    }
}

fn spawn_flush_worker(shared: Arc<SharedRootIndex>) -> FlushWorkerHandle {
    let signal = shared.flush_signal.clone();
    let signal_for_thread = signal.clone();
    let join_handle = thread::spawn(move || loop {
        match signal_for_thread.wait_for_flush() {
            FlushDecision::Shutdown => break,
            FlushDecision::Flush => {
                let flush_result = match shared.data.read() {
                    Ok(data) => write_index_snapshot(&shared, &data),
                    Err(_) => Err(lock_poisoned_error("filesystem index data")),
                };

                if let Err(error) = flush_result {
                    if let Ok(mut data) = shared.data.write() {
                        data.errors += 1;
                    }
                    log::warn!(
                        "filesystem index cache write failed for {}: {}",
                        shared.root.display(),
                        error
                    );
                }
            }
        }
    });

    FlushWorkerHandle::new(signal, join_handle)
}

/// Manager for filesystem indexes.
#[derive(Debug, Default)]
pub struct FileSystemIndexManager {
    indexes: RwLock<HashMap<RootIndexKey, Arc<RootIndex>>>,
    index_init_lane: Mutex<()>,
    search_version_tracker: SearchVersionTracker,
    unversioned_search_tracker: SearchVersionTracker,
    search_lane: Mutex<()>,
}

impl FileSystemIndexManager {
    /// Returns the next search version, cancelling any in-flight searches.
    ///
    /// Call this before starting a new search to get a version number.
    /// Pass this version to `search()` to enable cancellation.
    pub fn next_search_version(&self) -> u64 {
        self.search_version_tracker.next_version()
    }

    /// Returns the current search version without incrementing.
    pub fn current_search_version(&self) -> u64 {
        self.search_version_tracker.current_version()
    }

    fn build_key(&self, root: PathBuf, ignored_roots: Vec<PathBuf>) -> Result<RootIndexKey> {
        if !root.exists() {
            return Err(FilesystemError::InvalidInput(format!(
                "root path does not exist: {}",
                root.display()
            )));
        }
        let canonical_root = canonicalize_existing_path(root);
        let canonical_ignored_roots = ignored_roots
            .into_iter()
            .map(canonicalize_existing_path)
            .collect::<Vec<_>>();
        Ok(RootIndexKey::new(canonical_root, canonical_ignored_roots))
    }

    /// Searches the filesystem index.
    ///
    /// If `search_version` is omitted, a fresh version is allocated automatically.
    /// This keeps unversioned callers (tooling/internal callers) cancellable and
    /// avoids piling up concurrent expensive searches.
    #[allow(clippy::too_many_arguments)]
    pub fn search(
        &self,
        root: PathBuf,
        query: String,
        kind: KindFilter,
        include_hidden: bool,
        case_sensitive: bool,
        max_results: usize,
        max_depth: usize,
        cache_dir: PathBuf,
        ignored_roots: Vec<PathBuf>,
        search_version: Option<u64>,
    ) -> Result<Option<SearchResult>> {
        let key = self.build_key(root, ignored_roots)?;
        let index = self.get_or_create_index(key.clone(), cache_dir)?;

        ensure_build_started(index.shared.clone(), false);

        let (state, progress) = progress_snapshot(index.shared.as_ref());

        let cancel_token = if let Some(version) = search_version {
            self.search_version_tracker.token_for_version(version)
        } else {
            let version = self.unversioned_search_tracker.next_version();
            self.unversioned_search_tracker.token_for_version(version)
        };

        // Check if already cancelled before doing any work
        if cancel_token.is_cancelled().is_none() {
            return Ok(None);
        }

        // Cardinal executes searches on a single worker lane. Serializing search
        // execution here prevents CPU thrash and runaway parallel I/O under bursts.
        let _search_lane_guard = self
            .search_lane
            .lock()
            .map_err(|_| lock_poisoned_error("filesystem search lane"))?;

        if cancel_token.is_cancelled().is_none() {
            return Ok(None);
        }

        let data = index
            .shared
            .data
            .read()
            .map_err(|_| lock_poisoned_error("filesystem index data"))?;

        if state != "ready" && data.is_empty() {
            return Ok(Some(SearchResult {
                query,
                root: key.root.to_string_lossy().to_string(),
                entries: Vec::new(),
                count: 0,
                truncated: false,
                scanned: 0,
                errors: data.errors,
                index_state: state.to_string(),
                index_scanned_files: progress.scanned_files,
                index_scanned_dirs: progress.scanned_dirs,
                index_started_at: progress.started_at,
                index_last_update_at: progress.last_update_at,
                index_finished_at: progress.finished_at,
                highlight_terms: Vec::new(),
            }));
        }

        // Search returns None if cancelled
        search_index_data(
            &key.root,
            &data,
            query,
            kind,
            include_hidden,
            case_sensitive,
            max_results,
            max_depth,
            state,
            progress.scanned_files,
            progress.scanned_dirs,
            progress.started_at,
            progress.last_update_at,
            progress.finished_at,
            cancel_token,
        )
    }

    /// Returns the index status.
    pub fn index_status(
        &self,
        root: PathBuf,
        cache_dir: PathBuf,
        ignored_roots: Vec<PathBuf>,
    ) -> Result<IndexStatus> {
        let status_started = Instant::now();
        let key = self.build_key(root, ignored_roots)?;
        let index = self.get_or_create_index(key.clone(), cache_dir)?;

        ensure_build_started(index.shared.clone(), false);

        let status = build_status_payload(index.as_ref(), &key)?;

        if !index
            .first_status_timing_logged
            .swap(true, Ordering::Relaxed)
        {
            log::info!(
                "filesystem first index_status root={} elapsed_ms={} state={} indexed_entries={} scanned_files={} scanned_dirs={} watcher_enabled={}",
                key.root.display(),
                status_started.elapsed().as_millis(),
                status.state,
                status.indexed_entries,
                status.scanned_files,
                status.scanned_dirs,
                status.watcher_enabled,
            );
        }

        Ok(status)
    }

    /// Triggers a rescan of the index.
    pub fn rescan(
        &self,
        root: PathBuf,
        cache_dir: PathBuf,
        ignored_roots: Vec<PathBuf>,
    ) -> Result<IndexStatus> {
        let key = self.build_key(root, ignored_roots)?;
        let index = self.get_or_create_index(key.clone(), cache_dir)?;

        cancel_in_flight_build(index.shared.as_ref());

        let root_metadata = fs::symlink_metadata(&key.root).map_err(|error| {
            FilesystemError::InvalidInput(format!(
                "unable to access root path {}: {error}",
                key.root.display()
            ))
        })?;
        let root_is_dir = root_metadata.file_type().is_dir();

        let snapshot =
            build_index_snapshot_with_ignored_paths(&key.root, root_is_dir, &key.ignored_roots)?;

        {
            let mut data = index
                .shared
                .data
                .write()
                .map_err(|_| lock_poisoned_error("filesystem index data"))?;
            *data = snapshot;

            for _ in 0..2 {
                let pending = index
                    .shared
                    .pending_changes
                    .lock()
                    .ok()
                    .map(|mut guard| std::mem::take(&mut *guard))
                    .unwrap_or_default();
                if pending.is_empty() {
                    break;
                }
                for changed_path in coalesce_event_paths(pending) {
                    apply_path_change(index.shared.as_ref(), &mut data, &changed_path);
                }
            }
        }

        let finished_at = unix_now_secs();
        index
            .shared
            .build_state
            .store(IndexBuildState::Ready as u8, Ordering::Relaxed);
        index
            .shared
            .build_progress
            .scanned_files
            .store(0, Ordering::Relaxed);
        index
            .shared
            .build_progress
            .scanned_dirs
            .store(0, Ordering::Relaxed);
        index
            .shared
            .build_progress
            .started_at
            .store(finished_at, Ordering::Relaxed);
        index
            .shared
            .build_progress
            .last_update_at
            .store(finished_at, Ordering::Relaxed);
        index
            .shared
            .build_progress
            .finished_at
            .store(finished_at, Ordering::Relaxed);
        mark_index_dirty(index.shared.as_ref());

        build_status_payload(index.as_ref(), &key)
    }

    fn get_or_create_index(&self, key: RootIndexKey, cache_dir: PathBuf) -> Result<Arc<RootIndex>> {
        if let Some(existing) = self
            .indexes
            .read()
            .map_err(|_| lock_poisoned_error("filesystem index registry"))?
            .get(&key)
            .cloned()
        {
            return Ok(existing);
        }

        // Avoid holding the registry write lock while building/loading a root index.
        // This keeps unrelated calls from blocking behind heavy cache load work.
        let _init_lane_guard = self
            .index_init_lane
            .lock()
            .map_err(|_| lock_poisoned_error("filesystem index init lane"))?;

        if let Some(existing) = self
            .indexes
            .read()
            .map_err(|_| lock_poisoned_error("filesystem index registry"))?
            .get(&key)
            .cloned()
        {
            return Ok(existing);
        }

        let index = Arc::new(RootIndex::new(key.clone(), cache_dir)?);

        let mut indexes = self
            .indexes
            .write()
            .map_err(|_| lock_poisoned_error("filesystem index registry"))?;
        if let Some(existing) = indexes.get(&key).cloned() {
            return Ok(existing);
        }
        indexes.insert(key, index.clone());
        Ok(index)
    }
}

fn build_status_payload(index: &RootIndex, key: &RootIndexKey) -> Result<IndexStatus> {
    let state = IndexBuildState::load(&index.shared.build_state);
    let progress = index.shared.build_progress.snapshot();
    let (indexed_entries, errors) = match index.shared.data.try_read() {
        Ok(data) => (data.len(), data.errors),
        Err(TryLockError::WouldBlock) => (
            progress.scanned_files.saturating_add(progress.scanned_dirs),
            0,
        ),
        Err(TryLockError::Poisoned(_)) => {
            return Err(lock_poisoned_error("filesystem index data"));
        }
    };
    let watcher_enabled = match index._watcher.try_lock() {
        Ok(watcher) => watcher.is_some(),
        Err(TryLockError::WouldBlock) => false,
        Err(TryLockError::Poisoned(_)) => {
            return Err(lock_poisoned_error("filesystem watcher"));
        }
    };
    let last_error = index
        .shared
        .build_last_error
        .lock()
        .ok()
        .and_then(|guard| guard.clone());

    Ok(IndexStatus {
        state: state.as_str().to_string(),
        root: key.root.to_string_lossy().to_string(),
        ignored_paths: key
            .ignored_roots
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect(),
        indexed_entries,
        scanned_files: progress.scanned_files,
        scanned_dirs: progress.scanned_dirs,
        started_at: progress.started_at,
        last_update_at: progress.last_update_at,
        finished_at: progress.finished_at,
        errors,
        watcher_enabled,
        cache_path: index.shared.cache_path.to_string_lossy().to_string(),
        rescan_count: index.shared.rescan_count(),
        last_error,
    })
}

fn cancel_in_flight_build(shared: &SharedRootIndex) {
    // Set the cancellation flag for any in-flight build
    if let Ok(guard) = shared.build_cancel.lock() {
        if let Some(cancel_flag) = guard.as_ref() {
            cancel_flag.store(true, Ordering::SeqCst);
        }
    }
    // Also increment generation for tracking
    shared.build_generation.fetch_add(1, Ordering::SeqCst);
}

fn ensure_build_started(shared: Arc<SharedRootIndex>, force: bool) {
    if force {
        cancel_in_flight_build(shared.as_ref());
    }

    loop {
        let state = IndexBuildState::load(&shared.build_state);
        if !force && matches!(state, IndexBuildState::Building | IndexBuildState::Ready) {
            return;
        }

        let Ok(_) = shared.build_state.compare_exchange(
            state as u8,
            IndexBuildState::Building as u8,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) else {
            continue;
        };
        break;
    }

    let started_at = unix_now_secs();
    shared.build_progress.reset_for_build(started_at);
    if let Ok(mut last_error) = shared.build_last_error.lock() {
        *last_error = None;
    }

    // Create cancellation flag for this build
    let cancel_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));

    // Store in shared state so cancel_in_flight_build can set it
    if let Ok(mut guard) = shared.build_cancel.lock() {
        // Cancel any previous build
        if let Some(prev) = guard.take() {
            prev.store(true, Ordering::SeqCst);
        }
        *guard = Some(cancel_flag.clone());
    }

    // Increment generation for tracking
    shared.build_generation.fetch_add(1, Ordering::SeqCst);

    #[cfg(target_os = "macos")]
    shared
        .last_event_id
        .store(FsEventStream::current_event_id(), Ordering::Relaxed);

    let shared_for_thread = shared.clone();
    let cancel_flag_for_thread = cancel_flag.clone();

    thread::spawn(move || {
        // Catch panics to ensure we always update state even if the build thread panics
        let build_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            build_index_snapshot_with_ignored_paths_progress(
                &shared_for_thread.root,
                shared_for_thread.root_is_dir,
                &shared_for_thread.ignored_roots,
                &shared_for_thread.build_progress,
                &cancel_flag_for_thread,
            )
        }));

        let finished_at = unix_now_secs();
        shared_for_thread
            .build_progress
            .finished_at
            .store(finished_at, Ordering::Relaxed);

        // Handle panic case
        let result = match build_result {
            Ok(result) => result,
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "build thread panicked".to_string()
                };
                if let Ok(mut last_error) = shared_for_thread.build_last_error.lock() {
                    *last_error = Some(format!("panic during build: {panic_msg}"));
                }
                shared_for_thread
                    .build_state
                    .store(IndexBuildState::Error as u8, Ordering::Relaxed);
                return;
            }
        };

        // Check if this build was cancelled
        if cancel_flag_for_thread.load(Ordering::Relaxed) {
            shared_for_thread
                .build_state
                .store(IndexBuildState::Idle as u8, Ordering::Relaxed);
            return;
        }

        match result {
            Ok(Some(snapshot)) => {
                if let Ok(mut data) = shared_for_thread.data.write() {
                    *data = snapshot;

                    for _ in 0..2 {
                        let pending = shared_for_thread
                            .pending_changes
                            .lock()
                            .ok()
                            .map(|mut guard| std::mem::take(&mut *guard))
                            .unwrap_or_default();
                        if pending.is_empty() {
                            break;
                        }
                        for changed_path in coalesce_event_paths(pending) {
                            apply_path_change(shared_for_thread.as_ref(), &mut data, &changed_path);
                        }
                    }
                }
                shared_for_thread
                    .build_state
                    .store(IndexBuildState::Ready as u8, Ordering::Relaxed);
                mark_index_dirty(shared_for_thread.as_ref());
            }
            Ok(None) => {
                // Build was cancelled
                shared_for_thread
                    .build_state
                    .store(IndexBuildState::Idle as u8, Ordering::Relaxed);
            }
            Err(error) => {
                if let Ok(mut last_error) = shared_for_thread.build_last_error.lock() {
                    *last_error = Some(error.to_string());
                }
                shared_for_thread
                    .build_state
                    .store(IndexBuildState::Error as u8, Ordering::Relaxed);
            }
        }
    });
}

fn progress_snapshot(shared: &SharedRootIndex) -> (&'static str, crate::indexer::ProgressSnapshot) {
    let state = IndexBuildState::load(&shared.build_state).as_str();
    let progress = shared.build_progress.snapshot();
    (state, progress)
}

/// Builds index using Cardinal's two-phase approach with progress tracking.
///
/// Phase 1: Walk filesystem to build Node tree (children sorted during walk)
/// Phase 2: Convert tree to slab + name_index in single recursive pass
///
/// Returns `None` if cancelled.
fn build_index_snapshot_with_ignored_paths_progress(
    root: &Path,
    _root_is_dir: bool,
    ignored_roots: &[PathBuf],
    progress: &IndexBuildProgress,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<Option<RootIndexData>> {
    if !root.exists() {
        return Err(FilesystemError::InvalidInput(format!(
            "root path does not exist: {}",
            root.display()
        )));
    }

    // Cardinal-style two-phase build
    let walk_data = WalkData::new(root, ignored_roots)
        .with_cancel(cancel)
        .with_progress(progress)
        .with_file_metadata(false);

    // Phase 1 + 2: Walk builds tree, then RootIndexData::from_walk converts it
    let snapshot = RootIndexData::from_walk(&walk_data);

    progress
        .last_update_at
        .store(unix_now_secs(), Ordering::Relaxed);

    Ok(snapshot)
}

/// Builds index synchronously (for rescan operations).
fn build_index_snapshot_with_ignored_paths(
    root: &Path,
    _root_is_dir: bool,
    ignored_roots: &[PathBuf],
) -> Result<RootIndexData> {
    if !root.exists() {
        return Err(FilesystemError::InvalidInput(format!(
            "root path does not exist: {}",
            root.display()
        )));
    }

    // Cardinal-style two-phase build (no cancellation for sync builds)
    let walk_data = WalkData::new(root, ignored_roots).with_file_metadata(false);

    RootIndexData::from_walk(&walk_data).ok_or_else(|| {
        FilesystemError::Internal("index build was unexpectedly cancelled".to_string())
    })
}
