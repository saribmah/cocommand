//! FileSystemIndexManager - main API for filesystem indexing.
//!
//! Cardinal-style architecture: one index thread per root owns the data exclusively.
//! All communication happens through crossbeam channels — no shared mutable state.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use crossbeam_channel::{self as channel, Receiver, Sender};

use super::engine::search_index_data;
use crate::cancel::SearchVersionTracker;
use crate::error::{canonicalize_existing_path, FilesystemError, Result};
use crate::indexer::{
    load_index_snapshot, unix_now_secs, write_index_snapshot, FlushContext, IndexBuildProgress,
    IndexBuildState, RootIndexConfig, RootIndexData, RootIndexKey, WalkData, INDEX_FLUSH_IDLE,
    INDEX_FLUSH_MAX_DELAY, INDEX_FLUSH_POLL,
};
use crate::types::{IndexStatus, KindFilter, SearchResult};
use crate::watcher::{apply_path_change, coalesce_event_paths, WatcherEvent};

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

// ---------------------------------------------------------------------------
// Channel message types
// ---------------------------------------------------------------------------

struct SearchJob {
    query: String,
    kind: KindFilter,
    include_hidden: bool,
    case_sensitive: bool,
    max_results: usize,
    max_depth: usize,
    cancel_token: crate::cancel::CancellationToken,
    reply: Sender<Result<Option<SearchResult>>>,
}

struct RescanJob {
    reply: Sender<Result<IndexStatus>>,
}

// ---------------------------------------------------------------------------
// RootIndex — one per indexed root, owns the index thread
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct RootIndex {
    /// Channels for communicating with the index thread.
    search_tx: Sender<SearchJob>,
    rescan_tx: Sender<RescanJob>,

    /// Lock-free state reads (written by index thread, read by anyone).
    build_state: Arc<AtomicU8>,
    build_progress: Arc<IndexBuildProgress>,
    indexed_entries: Arc<AtomicUsize>,
    errors: Arc<AtomicUsize>,

    /// Config for status reporting.
    config: RootIndexConfig,
    watcher_enabled: Arc<AtomicBool>,
    last_error: Arc<Mutex<Option<String>>>,
    rescan_count: Arc<AtomicU64>,

    /// Keeps the watcher (FsEventStream on macOS) alive for the lifetime of this index.
    _watcher_slot: Arc<Mutex<Option<WatcherHandle>>>,

    first_status_timing_logged: AtomicBool,
}

impl std::fmt::Debug for RootIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RootIndex")
            .field("root", &self.config.root)
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

        let cache_load_started = Instant::now();
        let loaded_snapshot =
            load_index_snapshot(&cache_path, &key.root, root_is_dir, &key.ignored_roots);
        let cache_load_ms = cache_load_started.elapsed().as_millis();

        let (initial_data, initial_state, build_progress, cached_event_id, cache_loaded) =
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
        let cached_entries = initial_data.len();

        // Match Cardinal: report Ready immediately when we have a valid cache.
        // FSEvents history replay applies incremental updates in the background
        // without changing the visible state.
        let build_state_val = initial_state;

        #[cfg(target_os = "macos")]
        let since_event_id = if cached_event_id > 0 {
            cached_event_id
        } else {
            FsEventStream::current_event_id()
        };
        #[cfg(not(target_os = "macos"))]
        let _ = cached_event_id;

        let config = RootIndexConfig {
            root: key.root.clone(),
            root_is_dir,
            ignored_roots: key.ignored_roots.clone(),
            cache_path,
        };

        // Shared atomics for lock-free status reads
        let build_state = Arc::new(AtomicU8::new(build_state_val as u8));
        let build_progress = Arc::new(build_progress);
        let watcher_enabled = Arc::new(AtomicBool::new(false));
        let last_error: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let rescan_count = Arc::new(AtomicU64::new(0));
        #[cfg(target_os = "macos")]
        let last_event_id = Arc::new(AtomicU64::new(since_event_id));
        #[cfg(not(target_os = "macos"))]
        let last_event_id = Arc::new(AtomicU64::new(0));

        // Shared atomics for indexed_entries and errors (non-blocking status reads)
        let indexed_entries = Arc::new(AtomicUsize::new(initial_data.len()));
        let errors = Arc::new(AtomicUsize::new(0));

        // Create channels
        let (search_tx, search_rx) = channel::unbounded::<SearchJob>();
        let (rescan_tx, rescan_rx) = channel::unbounded::<RescanJob>();
        let (event_tx, event_rx) = channel::unbounded::<WatcherEvent>();

        // Spawn watcher (sends events through event_tx)
        let watcher_enabled_clone = watcher_enabled.clone();
        let last_error_clone = last_error.clone();
        let watcher_root = config.root.clone();
        let watcher_ignored = config.ignored_roots.clone();
        #[cfg(not(target_os = "macos"))]
        let watcher_root_is_dir = config.root_is_dir;
        let last_event_id_for_watcher = last_event_id.clone();

        // Keep a sender alive so event_rx never disconnects (even if watcher fails).
        let event_tx_keepalive = event_tx.clone();

        // Store watcher handle to keep it alive
        let watcher_slot: Arc<Mutex<Option<WatcherHandle>>> = Arc::new(Mutex::new(None));
        let watcher_slot_clone = watcher_slot.clone();

        thread::spawn(move || {
            #[cfg(target_os = "macos")]
            let watcher = match create_fsevent_watcher(
                &watcher_root,
                &watcher_ignored,
                since_event_id,
                event_tx.clone(),
                last_event_id_for_watcher,
            ) {
                Ok(watcher) => Some(watcher),
                Err(error) => {
                    let _ = last_error_clone.lock().map(|mut e| *e = Some(error.to_string()));
                    tracing::warn!(
                        "filesystem watcher disabled for {}: {}",
                        watcher_root.display(),
                        error
                    );
                    None
                }
            };

            #[cfg(not(target_os = "macos"))]
            let watcher = match create_index_watcher(
                watcher_root.clone(),
                watcher_root_is_dir,
                event_tx.clone(),
            ) {
                Ok(watcher) => Some(watcher),
                Err(error) => {
                    let _ = last_error_clone.lock().map(|mut e| *e = Some(error.to_string()));
                    tracing::warn!(
                        "filesystem watcher disabled for {}: {}",
                        watcher_root.display(),
                        error
                    );
                    None
                }
            };

            watcher_enabled_clone.store(watcher.is_some(), Ordering::Relaxed);
            if let Ok(mut slot) = watcher_slot_clone.lock() {
                *slot = watcher;
            }
        });

        // Spawn the index thread — sole owner of RootIndexData
        let idx_config = config.clone();
        let idx_build_state = build_state.clone();
        let idx_build_progress = build_progress.clone();
        let idx_last_error = last_error.clone();
        let idx_rescan_count = rescan_count.clone();
        let idx_last_event_id = last_event_id;
        let idx_indexed_entries = indexed_entries.clone();
        let idx_errors = errors.clone();

        thread::spawn(move || {
            // Hold event_tx_keepalive so the channel stays open for the thread's lifetime.
            let _event_tx = event_tx_keepalive;
            run_index_thread(
                idx_config,
                initial_data,
                build_state_val,
                search_rx,
                rescan_rx,
                event_rx,
                idx_build_state,
                idx_build_progress,
                idx_last_error,
                idx_rescan_count,
                idx_last_event_id,
                idx_indexed_entries,
                idx_errors,
            );
        });

        tracing::info!(
            "filesystem index init root={} cache_loaded={} cache_load_ms={} cached_entries={} state={} total_init_ms={}",
            key.root.display(),
            cache_loaded,
            cache_load_ms,
            cached_entries,
            build_state_val.as_str(),
            init_started.elapsed().as_millis(),
        );

        Ok(Self {
            search_tx,
            rescan_tx,
            build_state,
            build_progress,
            indexed_entries,
            errors,
            config,
            watcher_enabled,
            last_error,
            rescan_count,
            _watcher_slot: watcher_slot,
            first_status_timing_logged: AtomicBool::new(false),
        })
    }
}

// ---------------------------------------------------------------------------
// Index thread — sole owner of RootIndexData, processes all messages
// ---------------------------------------------------------------------------

fn run_index_thread(
    config: RootIndexConfig,
    mut data: RootIndexData,
    initial_state: IndexBuildState,
    search_rx: Receiver<SearchJob>,
    rescan_rx: Receiver<RescanJob>,
    event_rx: Receiver<WatcherEvent>,
    build_state: Arc<AtomicU8>,
    build_progress: Arc<IndexBuildProgress>,
    last_error: Arc<Mutex<Option<String>>>,
    rescan_count: Arc<AtomicU64>,
    last_event_id: Arc<AtomicU64>,
    indexed_entries: Arc<AtomicUsize>,
    errors: Arc<AtomicUsize>,
) {
    let flush_tick = channel::tick(INDEX_FLUSH_POLL);
    let mut dirty = false;
    let mut first_dirty_at: Option<Instant> = None;
    let mut last_search_at: Option<Instant> = None;
    let mut pending_during_build: Vec<PathBuf> = Vec::new();

    // Channel for receiving build results from worker threads.
    let (build_done_tx, build_done_rx) = channel::bounded::<Option<RootIndexData>>(1);

    // Pending rescan reply senders — we'll reply once the build completes.
    let mut pending_rescan_replies: Vec<Sender<Result<IndexStatus>>> = Vec::new();

    // If we need to build from scratch, kick off async build
    if matches!(initial_state, IndexBuildState::Idle | IndexBuildState::Error) {
        start_async_build(
            &config,
            &build_state,
            &build_progress,
            &last_error,
            build_done_tx.clone(),
        );
    }

    // Cardinal-style event loop: process one batch per select iteration inline.
    // FSEvents delivers batches at ~100ms intervals. coalesce_event_paths reduces
    // each batch to a minimal set of root paths, then we walk each. After one
    // batch, the select loop goes around and search/rescan can be serviced.
    loop {
        channel::select! {
            recv(search_rx) -> job => {
                let Ok(job) = job else { break };
                last_search_at = Some(Instant::now());
                let result = execute_search(&config, &data, &build_state, &build_progress, job.query, job.kind, job.include_hidden, job.case_sensitive, job.max_results, job.max_depth, job.cancel_token);
                let _ = job.reply.send(result);
            }
            recv(rescan_rx) -> job => {
                let Ok(job) = job else { break };
                if IndexBuildState::load(&build_state) == IndexBuildState::Building {
                    pending_rescan_replies.push(job.reply);
                } else {
                    start_async_build(
                        &config,
                        &build_state,
                        &build_progress,
                        &last_error,
                        build_done_tx.clone(),
                    );
                    pending_rescan_replies.push(job.reply);
                }
            }
            recv(build_done_rx) -> result => {
                let Ok(result) = result else { break };
                match result {
                    Some(snapshot) => {
                        data = snapshot;
                        indexed_entries.store(data.len(), Ordering::Relaxed);
                        let finished_at = unix_now_secs();
                        build_progress.finished_at.store(finished_at, Ordering::Relaxed);
                        build_progress.last_update_at.store(finished_at, Ordering::Relaxed);
                        build_state.store(IndexBuildState::Ready as u8, Ordering::Relaxed);
                        tracing::info!(
                            "filesystem index build complete root={} entries={}",
                            config.root.display(),
                            data.len(),
                        );
                    }
                    None => {
                        build_state.store(IndexBuildState::Error as u8, Ordering::Relaxed);
                        if let Ok(mut e) = last_error.lock() {
                            *e = Some("index build was unexpectedly cancelled".to_string());
                        }
                    }
                }

                // Apply any events that arrived during the build
                drain_pending_events(&event_rx, &mut pending_during_build);
                apply_pending_paths(&config, &mut data, &mut pending_during_build);
                indexed_entries.store(data.len(), Ordering::Relaxed);
                mark_dirty(&mut dirty, &mut first_dirty_at);

                // Reply to any queued rescan requests
                for reply in pending_rescan_replies.drain(..) {
                    let status = build_status_payload_from_atomics(&config, &build_state, &build_progress, &last_error, &rescan_count, &indexed_entries, &errors);
                    let _ = reply.send(status);
                }
            }
            recv(event_rx) -> event => {
                let Ok(event) = event else { break };
                match event {
                    WatcherEvent::PathsChanged(paths) => {
                        if IndexBuildState::load(&build_state) == IndexBuildState::Building {
                            pending_during_build.extend(paths);
                        } else {
                            // Coalesce this batch to a minimal cover set (identical
                            // to Cardinal's scan_paths), then apply inline. Each
                            // FSEvents batch is ~100ms of events so the coalesced
                            // set is small — the select loop stays responsive.
                            for changed_path in coalesce_event_paths(paths) {
                                // tracing::info!("Scanning path: {:?}", changed_path);
                                apply_path_change(&config.root, config.root_is_dir, &config.ignored_roots, &mut data, &changed_path);
                            }
                            indexed_entries.store(data.len(), Ordering::Relaxed);
                            mark_dirty(&mut dirty, &mut first_dirty_at);
                        }
                    }
                    WatcherEvent::RescanRequired => {
                        rescan_count.fetch_add(1, Ordering::Relaxed);
                        if IndexBuildState::load(&build_state) != IndexBuildState::Building {
                            start_async_build(
                                &config,
                                &build_state,
                                &build_progress,
                                &last_error,
                                build_done_tx.clone(),
                            );
                        }
                    }
                    WatcherEvent::HistoryDone => {
                        tracing::info!(
                            "filesystem history replay complete root={}",
                            config.root.display(),
                        );
                    }
                    WatcherEvent::Error(msg) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                        if let Ok(mut e) = last_error.lock() {
                            *e = Some(msg);
                        }
                    }
                }
            }
            recv(flush_tick) -> _ => {
                if dirty {
                    let now = Instant::now();
                    let search_idle = last_search_at
                        .map(|t| now.duration_since(t) >= INDEX_FLUSH_IDLE)
                        .unwrap_or(true);
                    let max_delay_ok = first_dirty_at
                        .map(|t| now.duration_since(t) >= INDEX_FLUSH_MAX_DELAY)
                        .unwrap_or(false);

                    if search_idle || max_delay_ok {
                        do_flush(&config, &data, &last_event_id, &rescan_count);
                        dirty = false;
                        first_dirty_at = None;
                    }
                }
            }
        }
    }
}

/// Spawns a build on a worker thread, sends result back through `done_tx`.
fn start_async_build(
    config: &RootIndexConfig,
    build_state: &Arc<AtomicU8>,
    build_progress: &Arc<IndexBuildProgress>,
    last_error: &Arc<Mutex<Option<String>>>,
    done_tx: Sender<Option<RootIndexData>>,
) {
    build_state.store(IndexBuildState::Building as u8, Ordering::Relaxed);
    let started_at = unix_now_secs();
    build_progress.reset_for_build(started_at);
    if let Ok(mut e) = last_error.lock() {
        *e = None;
    }

    let build_config = config.clone();
    let build_progress = build_progress.clone();
    thread::spawn(move || {
        let walk_data = WalkData::new(&build_config.root, &build_config.ignored_roots)
            .with_progress(&build_progress)
            .with_file_metadata(false);

        let result = RootIndexData::from_walk(&walk_data);
        let _ = done_tx.send(result);
    });
}

fn mark_dirty(dirty: &mut bool, first_dirty_at: &mut Option<Instant>) {
    if !*dirty {
        *first_dirty_at = Some(Instant::now());
    }
    *dirty = true;
}

fn drain_pending_events(event_rx: &Receiver<WatcherEvent>, pending: &mut Vec<PathBuf>) {
    while let Ok(event) = event_rx.try_recv() {
        if let WatcherEvent::PathsChanged(paths) = event {
            pending.extend(paths);
        }
    }
}

fn apply_pending_paths(config: &RootIndexConfig, data: &mut RootIndexData, pending: &mut Vec<PathBuf>) {
    if pending.is_empty() {
        return;
    }
    let paths = std::mem::take(pending);
    for changed_path in coalesce_event_paths(paths) {
        apply_path_change(&config.root, config.root_is_dir, &config.ignored_roots, data, &changed_path);
    }
}

fn do_flush(
    config: &RootIndexConfig,
    data: &RootIndexData,
    last_event_id: &AtomicU64,
    rescan_count: &AtomicU64,
) {
    let ctx = FlushContext {
        root: &config.root,
        root_is_dir: config.root_is_dir,
        ignored_roots: &config.ignored_roots,
        cache_path: &config.cache_path,
        last_event_id: last_event_id.load(Ordering::Relaxed),
        rescan_count: rescan_count.load(Ordering::Relaxed),
    };
    if let Err(error) = write_index_snapshot(&ctx, data) {
        tracing::warn!(
            "filesystem index cache write failed for {}: {}",
            config.root.display(),
            error
        );
    }
}

#[tracing::instrument(skip_all, fields(query = %query))]
fn execute_search(
    config: &RootIndexConfig,
    data: &RootIndexData,
    build_state: &AtomicU8,
    build_progress: &IndexBuildProgress,
    query: String,
    kind: KindFilter,
    include_hidden: bool,
    case_sensitive: bool,
    max_results: usize,
    max_depth: usize,
    cancel_token: crate::cancel::CancellationToken,
) -> Result<Option<SearchResult>> {
    let state = IndexBuildState::load(build_state).as_str();
    let progress = build_progress.snapshot();

    if cancel_token.is_cancelled().is_none() {
        return Ok(None);
    }

    if state != "ready" && data.is_empty() {
        return Ok(Some(SearchResult {
            query,
            root: config.root.to_string_lossy().to_string(),
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

    search_index_data(
        &config.root,
        data,
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

fn build_status_payload_from_atomics(
    config: &RootIndexConfig,
    build_state: &AtomicU8,
    build_progress: &IndexBuildProgress,
    last_error: &Mutex<Option<String>>,
    rescan_count: &AtomicU64,
    indexed_entries: &AtomicUsize,
    errors: &AtomicUsize,
) -> Result<IndexStatus> {
    let state = IndexBuildState::load(build_state);
    let progress = build_progress.snapshot();

    let last_error_msg = last_error
        .lock()
        .ok()
        .and_then(|guard| guard.clone());

    Ok(IndexStatus {
        state: state.as_str().to_string(),
        root: config.root.to_string_lossy().to_string(),
        ignored_paths: config
            .ignored_roots
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect(),
        indexed_entries: indexed_entries.load(Ordering::Relaxed),
        scanned_files: progress.scanned_files,
        scanned_dirs: progress.scanned_dirs,
        started_at: progress.started_at,
        last_update_at: progress.last_update_at,
        finished_at: progress.finished_at,
        errors: errors.load(Ordering::Relaxed),
        watcher_enabled: true,
        cache_path: config.cache_path.to_string_lossy().to_string(),
        rescan_count: rescan_count.load(Ordering::Relaxed),
        last_error: last_error_msg,
    })
}

// ---------------------------------------------------------------------------
// Public API — FileSystemIndexManager
// ---------------------------------------------------------------------------

/// Manager for filesystem indexes.
///
/// This is the public API consumed by extensions. All operations are non-blocking
/// from the caller's perspective — they send a message to the index thread and
/// wait for a reply on a bounded channel.
#[derive(Debug)]
pub struct FileSystemIndexManager {
    indexes: Mutex<HashMap<RootIndexKey, Arc<RootIndex>>>,
    /// Serializes `RootIndex::new()` calls so only one thread pays the cache-load cost.
    creation_lock: Mutex<()>,
    search_version_tracker: SearchVersionTracker,
    unversioned_search_tracker: SearchVersionTracker,
}

impl Default for FileSystemIndexManager {
    fn default() -> Self {
        Self {
            indexes: Mutex::new(HashMap::new()),
            creation_lock: Mutex::new(()),
            search_version_tracker: SearchVersionTracker::default(),
            unversioned_search_tracker: SearchVersionTracker::default(),
        }
    }
}

impl FileSystemIndexManager {
    /// Returns the next search version, cancelling any in-flight searches.
    pub fn next_search_version(&self) -> u64 {
        self.search_version_tracker.next_version()
    }

    /// Returns the current search version without incrementing.
    pub fn current_search_version(&self) -> u64 {
        self.search_version_tracker.current_version()
    }

    /// Returns the build state of the most recently created root index
    /// without triggering a build. Returns `Idle` if no roots exist.
    pub fn peek_build_state(&self) -> IndexBuildState {
        let indexes = match self.indexes.lock() {
            Ok(guard) => guard,
            Err(_) => return IndexBuildState::Idle,
        };
        indexes
            .values()
            .next()
            .map(|index| IndexBuildState::load(&index.build_state))
            .unwrap_or(IndexBuildState::Idle)
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
        let index = self.get_or_create_index(key, cache_dir)?;

        let cancel_token = if let Some(version) = search_version {
            self.search_version_tracker.activate_version(version);
            self.search_version_tracker.token_for_version(version)
        } else {
            let version = self.unversioned_search_tracker.next_version();
            self.unversioned_search_tracker.token_for_version(version)
        };

        if cancel_token.is_cancelled().is_none() {
            return Ok(None);
        }

        let (reply_tx, reply_rx) = channel::bounded(1);
        let job = SearchJob {
            query,
            kind,
            include_hidden,
            case_sensitive,
            max_results,
            max_depth,
            cancel_token,
            reply: reply_tx,
        };

        index.search_tx.send(job).map_err(|_| {
            FilesystemError::Internal("index thread has shut down".to_string())
        })?;

        reply_rx.recv().map_err(|_| {
            FilesystemError::Internal("index thread did not reply".to_string())
        })?
    }

    /// Returns the index status, triggering a build if needed.
    ///
    /// This is fully non-blocking — reads shared atomics directly without
    /// sending a message to the index thread.
    pub fn index_status(
        &self,
        root: PathBuf,
        cache_dir: PathBuf,
        ignored_roots: Vec<PathBuf>,
    ) -> Result<IndexStatus> {
        let status_started = Instant::now();
        let key = self.build_key(root, ignored_roots)?;
        let index = self.get_or_create_index(key.clone(), cache_dir)?;

        let status = build_status_payload_from_atomics(
            &index.config,
            &index.build_state,
            &index.build_progress,
            &index.last_error,
            &index.rescan_count,
            &index.indexed_entries,
            &index.errors,
        )?;

        if !index
            .first_status_timing_logged
            .swap(true, Ordering::Relaxed)
        {
            tracing::info!(
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
        let index = self.get_or_create_index(key, cache_dir)?;

        let (reply_tx, reply_rx) = channel::bounded(1);
        index.rescan_tx.send(RescanJob { reply: reply_tx }).map_err(|_| {
            FilesystemError::Internal("index thread has shut down".to_string())
        })?;

        reply_rx.recv().map_err(|_| {
            FilesystemError::Internal("index thread did not reply".to_string())
        })?
    }

    fn get_or_create_index(&self, key: RootIndexKey, cache_dir: PathBuf) -> Result<Arc<RootIndex>> {
        // Fast path: check if already exists (brief lock).
        {
            let indexes = self.indexes.lock().map_err(|_| {
                FilesystemError::Internal("filesystem index registry lock poisoned".to_string())
            })?;
            if let Some(existing) = indexes.get(&key).cloned() {
                return Ok(existing);
            }
        }

        // Serialize creation — only one thread pays the cache-load cost.
        let _creation_guard = self.creation_lock.lock().map_err(|_| {
            FilesystemError::Internal("filesystem index creation lock poisoned".to_string())
        })?;

        // Re-check under creation lock — another thread may have created it while we waited.
        {
            let indexes = self.indexes.lock().map_err(|_| {
                FilesystemError::Internal("filesystem index registry lock poisoned".to_string())
            })?;
            if let Some(existing) = indexes.get(&key).cloned() {
                return Ok(existing);
            }
        }

        // Create index outside the indexes lock (cache load can be slow).
        let index = Arc::new(RootIndex::new(key.clone(), cache_dir)?);

        let mut indexes = self.indexes.lock().map_err(|_| {
            FilesystemError::Internal("filesystem index registry lock poisoned".to_string())
        })?;
        indexes.insert(key, index.clone());
        Ok(index)
    }
}
