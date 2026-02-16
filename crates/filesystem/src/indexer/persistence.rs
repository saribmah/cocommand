//! Index persistence - cache read/write operations.
//!
//! This module implements persistence using direct slab serialization
//! with postcard encoding and zstd compression. The format stores the slab directly
//! without converting to intermediate node representations, making both read and
//! write operations significantly faster.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::thread::available_parallelism;
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use super::build::unix_now_secs;
use super::data::RootIndexData;
use super::file_nodes::FileNodes;
use super::shared::SharedRootIndex;
use crate::error::{FilesystemError, Result};
use crate::storage::{SlabIndex, SlabNode, SortedSlabIndices, ThinSlab, NAME_POOL};

/// Cache format version - increment when changing the format.
/// Version 7: direct slab persistence with postcard encoding.
pub const INDEX_CACHE_VERSION: u32 = 7;

/// Maximum age of cache before it's considered stale (non-macOS only).
/// On macOS, FSEvents replay makes this unnecessary when we have a saved event ID.
/// This is kept for non-macOS platforms where we can't rely on event replay.
pub const INDEX_CACHE_MAX_AGE_SECS: u64 = 60 * 60;

/// Persistent storage format for the filesystem index.
///
/// The slab and name_index are serialized directly without conversion to intermediate types.
#[derive(Serialize, Deserialize)]
pub struct PersistentStorage {
    /// Cache format version.
    pub version: u32,
    /// Last FSEvents event ID (for incremental updates on macOS).
    pub last_event_id: u64,
    /// Root file path of the cache.
    pub path: PathBuf,
    /// Whether the root is a directory.
    pub root_is_dir: bool,
    /// Paths to ignore during indexing.
    pub ignore_paths: Vec<PathBuf>,
    /// Root node index in the slab.
    pub slab_root: SlabIndex,
    /// The slab containing all nodes (directly serialized).
    pub slab: ThinSlab<SlabNode>,
    /// Name to indices mapping (directly serialized).
    /// Uses `Box<str>` keys because `&'static str` can't be deserialized directly.
    pub name_index: BTreeMap<Box<str>, SortedSlabIndices>,
    /// Number of rescans performed before this snapshot.
    pub rescan_count: u64,
    /// Timestamp when the cache was saved.
    pub saved_at: u64,
    /// Error count during indexing.
    pub errors: usize,
}

// ---------------------------------------------------------------------------
// Key for identifying a root index
// ---------------------------------------------------------------------------

/// Key for identifying a root index.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct RootIndexKey {
    pub root: PathBuf,
    pub ignored_roots: Vec<PathBuf>,
}

impl RootIndexKey {
    /// Creates a new root index key.
    pub fn new(root: PathBuf, mut ignored_roots: Vec<PathBuf>) -> Self {
        ignored_roots.sort();
        ignored_roots.dedup();
        Self {
            root,
            ignored_roots,
        }
    }

    /// Returns the cache file path for this key.
    pub fn cache_path(&self, cache_dir: &Path) -> PathBuf {
        cache_dir.join(format!("fs-index-{}.bin.zst", cache_key_fingerprint(self)))
    }
}

// ---------------------------------------------------------------------------
// Write operations
// ---------------------------------------------------------------------------

/// Writes an index snapshot to the cache file.
///
/// approach:
/// - Postcard encoding (compact binary format)
/// - Zstd compression (level 6, multi-threaded)
/// - Atomic write (temp file + rename)
pub fn write_index_snapshot(shared: &SharedRootIndex, data: &RootIndexData) -> Result<()> {
    // Convert name_index keys to Box<str> for serialization
    // (can't serialize &'static str directly)
    let name_index: BTreeMap<Box<str>, SortedSlabIndices> = data
        .name_index
        .iter()
        .map(|(name, indices)| ((*name).into(), indices.clone()))
        .collect();

    let storage = PersistentStorage {
        version: INDEX_CACHE_VERSION,
        last_event_id: shared.last_event_id.load(Ordering::Relaxed),
        path: shared.root.clone(),
        root_is_dir: shared.root_is_dir,
        ignore_paths: shared.ignored_roots.clone(),
        slab_root: data.file_nodes.root(),
        // Clone the slab while preserving sparse indices. Cardinal keeps slab
        // indices stable across updates; compacting would corrupt parent/child
        // links and name-index references after deletions.
        slab: clone_slab_for_persistence(&data.file_nodes)?,
        name_index,
        rescan_count: shared.rescan_count(),
        saved_at: unix_now_secs(),
        errors: data.errors,
    };

    // Ensure cache directory exists
    if let Some(parent) = shared.cache_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            FilesystemError::Internal(format!(
                "failed to create filesystem cache directory {}: {error}",
                parent.display()
            ))
        })?;
    }

    // Write to temp file first for atomic operation
    let tmp_path = shared.cache_path.with_extension("tmp");

    {
        let output = File::create(&tmp_path).map_err(|error| {
            FilesystemError::Internal(format!(
                "failed to create cache file {}: {error}",
                tmp_path.display()
            ))
        })?;

        // Zstd encoder with level 6 and multi-threading
        let mut encoder = zstd::Encoder::new(output, 6).map_err(|error| {
            FilesystemError::Internal(format!("failed to create zstd encoder: {error}"))
        })?;

        // Enable multi-threaded compression for better performance on large indexes
        let threads = available_parallelism().map(|x| x.get() as u32).unwrap_or(4);
        encoder.multithread(threads).map_err(|error| {
            FilesystemError::Internal(format!("failed to enable multi-threaded zstd: {error}"))
        })?;

        let output = encoder.auto_finish();
        let mut output = BufWriter::new(output);

        // Serialize with postcard
        postcard::to_io(&storage, &mut output).map_err(|error| {
            FilesystemError::Internal(format!("failed to encode cache with postcard: {error}"))
        })?;
    }

    // Atomic rename
    fs::rename(&tmp_path, &shared.cache_path).map_err(|error| {
        FilesystemError::Internal(format!(
            "failed to finalize filesystem cache file {}: {error}",
            shared.cache_path.display()
        ))
    })?;

    log::debug!(
        "wrote filesystem cache to {} ({} nodes)",
        shared.cache_path.display(),
        data.file_nodes.len()
    );

    Ok(())
}

/// Clones the slab for persistence.
///
/// This creates a new ThinSlab with all nodes copied. The serialization
/// will then encode the slab contents directly.
fn clone_slab_for_persistence(file_nodes: &FileNodes) -> Result<ThinSlab<SlabNode>> {
    // Round-trip through postcard to preserve exact sparse indices.
    // ThinSlab's serde impl stores occupied entries keyed by index.
    let encoded = postcard::to_stdvec(&**file_nodes).map_err(|error| {
        FilesystemError::Internal(format!(
            "failed to serialize slab for snapshot clone: {error}"
        ))
    })?;
    postcard::from_bytes(&encoded).map_err(|error| {
        FilesystemError::Internal(format!(
            "failed to deserialize slab for snapshot clone: {error}"
        ))
    })
}

// ---------------------------------------------------------------------------
// Read operations
// ---------------------------------------------------------------------------

/// Loads an index snapshot from the cache file.
///
/// Returns `(RootIndexData, saved_at, last_event_id)` on success.
pub fn load_index_snapshot(
    cache_path: &Path,
    root: &Path,
    root_is_dir: bool,
    ignored_roots: &[PathBuf],
) -> Option<(RootIndexData, u64, u64)> {
    // Read and decompress
    let input = match File::open(cache_path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return None,
        Err(error) => {
            log::warn!(
                "filesystem cache read failed for {}: {}",
                cache_path.display(),
                error
            );
            return None;
        }
    };

    let decoder = match zstd::Decoder::new(input) {
        Ok(d) => d,
        Err(error) => {
            log::warn!(
                "filesystem cache decompress failed for {}: {}",
                cache_path.display(),
                error
            );
            return None;
        }
    };

    let mut input = BufReader::new(decoder);
    let mut scratch = vec![0u8; 4 * 1024];

    let storage: PersistentStorage = match postcard::from_io((&mut input, &mut scratch)) {
        Ok((s, _)) => s,
        Err(error) => {
            log::warn!(
                "filesystem cache decode failed for {}: {}",
                cache_path.display(),
                error
            );
            return None;
        }
    };

    // Validate version
    if storage.version != INDEX_CACHE_VERSION {
        log::debug!(
            "cache version mismatch: {} != {}",
            storage.version,
            INDEX_CACHE_VERSION
        );
        return None;
    }

    // Validate root path
    if storage.path != root {
        log::debug!("cache root mismatch: {:?} != {:?}", storage.path, root);
        return None;
    }

    // Validate root_is_dir
    if storage.root_is_dir != root_is_dir {
        log::debug!(
            "cache root_is_dir mismatch: {} != {}",
            storage.root_is_dir,
            root_is_dir
        );
        return None;
    }

    // Validate ignore paths
    if storage.ignore_paths != ignored_roots {
        log::debug!("cache ignore_paths mismatch");
        return None;
    }

    if !storage_graph_is_consistent(&storage) {
        log::warn!(
            "filesystem cache graph validation failed for {}, rebuilding",
            cache_path.display()
        );
        return None;
    }

    let last_event_id = storage.last_event_id;
    let saved_at = storage.saved_at;

    // On macOS with a saved event ID, trust FSEvents to replay
    // any missed events since last_event_id. This allows the cache to be used
    // regardless of age - FSEvents will bring it up to date incrementally.
    //
    // On non-macOS platforms (or when event ID is missing), fall back to
    // TTL + mtime staleness checks since we can't replay missed events.
    #[cfg(target_os = "macos")]
    let needs_staleness_check = last_event_id == 0;
    #[cfg(not(target_os = "macos"))]
    let needs_staleness_check = true;

    if needs_staleness_check && cache_is_stale(root, saved_at) {
        return None;
    }

    // Reconstruct RootIndexData from persisted storage
    let data = restore_from_storage(storage, root);

    log::debug!(
        "loaded filesystem cache from {} ({} nodes, event_id={})",
        cache_path.display(),
        data.file_nodes.len(),
        last_event_id
    );

    Some((data, saved_at, last_event_id))
}

/// Restores RootIndexData from PersistentStorage.
///
/// only file_nodes and name_index are stored,
/// no secondary indexes need to be rebuilt.
fn restore_from_storage(storage: PersistentStorage, _root: &Path) -> RootIndexData {
    // Create FileNodes from the deserialized slab
    let file_nodes = FileNodes::new(storage.slab, storage.slab_root);

    // Reconstruct name_index with interned keys
    // During deserialization, names are already re-interned in NAME_POOL
    // via NameAndParent's Deserialize impl
    let mut name_index: BTreeMap<&'static str, SortedSlabIndices> = BTreeMap::new();
    for (boxed_name, indices) in storage.name_index {
        let interned = NAME_POOL.intern(&boxed_name);
        name_index.insert(interned, indices);
    }

    RootIndexData {
        file_nodes,
        name_index,
        errors: storage.errors,
    }
}

/// Validates that all slab references are internally consistent.
///
/// This guards against corrupted cache snapshots where node indices were
/// compacted without remapping parent/child/name-index references.
fn storage_graph_is_consistent(storage: &PersistentStorage) -> bool {
    // Root must exist.
    if storage.slab.get(storage.slab_root).is_none() {
        return false;
    }

    // Name-index entries must point to existing slab nodes.
    for indices in storage.name_index.values() {
        if indices.iter().any(|idx| storage.slab.get(*idx).is_none()) {
            return false;
        }
    }

    // Validate reachable tree from root:
    // - every child index exists
    // - child parent points back to current node
    // - no cycles in child graph
    let mut visited = std::collections::BTreeSet::new();
    let mut on_stack = std::collections::BTreeSet::new();
    let mut stack = vec![(storage.slab_root, 0usize)];
    on_stack.insert(storage.slab_root);

    while let Some((current_id, child_idx)) = stack.last_mut() {
        let Some(current_node) = storage.slab.get(*current_id) else {
            return false;
        };

        if *child_idx >= current_node.children.len() {
            on_stack.remove(current_id);
            visited.insert(*current_id);
            stack.pop();
            continue;
        }

        let child_id = current_node.children[*child_idx];
        *child_idx += 1;

        let Some(child_node) = storage.slab.get(child_id) else {
            return false;
        };
        if child_node.parent() != Some(*current_id) {
            return false;
        }
        if on_stack.contains(&child_id) {
            return false;
        }
        if !visited.contains(&child_id) {
            on_stack.insert(child_id);
            stack.push((child_id, 0));
        }
    }

    // All slab nodes should be reachable from the declared root.
    visited.len() == storage.slab.len()
}

// ---------------------------------------------------------------------------
// Staleness checks
// ---------------------------------------------------------------------------

/// Checks if the cache is stale (for non-macOS or missing event ID).
///
/// Uses two heuristics:
/// 1. TTL: Cache older than INDEX_CACHE_MAX_AGE_SECS is considered stale
/// 2. Mtime: If root directory was modified after cache was saved, consider stale
///
/// Note: On macOS with a valid last_event_id, this function is not called
/// because FSEvents can replay missed events to bring the cache up to date.
fn cache_is_stale(root: &Path, saved_at: u64) -> bool {
    let now = unix_now_secs();
    if now.saturating_sub(saved_at) > INDEX_CACHE_MAX_AGE_SECS {
        log::debug!(
            "cache stale: age {} secs > max {} secs",
            now.saturating_sub(saved_at),
            INDEX_CACHE_MAX_AGE_SECS
        );
        return true;
    }

    let Ok(metadata) = fs::symlink_metadata(root) else {
        log::debug!("cache stale: cannot read root metadata");
        return true;
    };
    let modified = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_secs())
        .unwrap_or(0);
    if modified > saved_at {
        log::debug!(
            "cache stale: root mtime {} > saved_at {}",
            modified,
            saved_at
        );
        return true;
    }
    false
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Computes a fingerprint for the cache key.
fn cache_key_fingerprint(key: &RootIndexKey) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    fnv1a_update(&mut hash, key.root.to_string_lossy().as_bytes());
    fnv1a_update(&mut hash, &[0xff]);
    for ignored in &key.ignored_roots {
        fnv1a_update(&mut hash, ignored.to_string_lossy().as_bytes());
        fnv1a_update(&mut hash, &[0xfe]);
    }
    format!("{hash:016x}")
}

fn fnv1a_update(hash: &mut u64, bytes: &[u8]) {
    const FNV_PRIME: u64 = 0x100000001b3;
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use crate::indexer::WalkData;

    #[test]
    fn test_cache_key_fingerprint() {
        let key = RootIndexKey::new(PathBuf::from("/Users/test"), vec![]);
        let fp = cache_key_fingerprint(&key);
        assert!(!fp.is_empty());
        assert_eq!(fp.len(), 16); // 64-bit hex

        // Same key should produce same fingerprint
        let key2 = RootIndexKey::new(PathBuf::from("/Users/test"), vec![]);
        assert_eq!(cache_key_fingerprint(&key), cache_key_fingerprint(&key2));

        // Different key should produce different fingerprint
        let key3 = RootIndexKey::new(PathBuf::from("/Users/other"), vec![]);
        assert_ne!(cache_key_fingerprint(&key), cache_key_fingerprint(&key3));
    }

    #[test]
    fn test_root_index_key_deduplication() {
        let key = RootIndexKey::new(
            PathBuf::from("/test"),
            vec![
                PathBuf::from("/a"),
                PathBuf::from("/b"),
                PathBuf::from("/a"), // duplicate
            ],
        );
        assert_eq!(key.ignored_roots.len(), 2);
    }

    #[test]
    fn clone_slab_preserves_sparse_indices() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let root = temp.path();
        fs::write(root.join("a.txt"), b"a").expect("write a");
        fs::write(root.join("b.txt"), b"b").expect("write b");

        let walk_data = WalkData::new(root, &[]);
        let mut data = crate::indexer::RootIndexData::from_walk(&walk_data).expect("walk");

        // Remove one sibling to create a hole in the slab index space.
        data.remove_entry(&root.join("a.txt"));
        let b_idx = data
            .node_index_for_path(&root.join("b.txt"))
            .expect("b should exist");

        let cloned = clone_slab_for_persistence(&data.file_nodes).expect("clone slab");

        // The existing node index must still resolve after cloning.
        assert!(
            cloned.get(b_idx).is_some(),
            "sparse slab index must remain valid after clone"
        );
    }

    #[test]
    fn storage_graph_validation_rejects_missing_child_reference() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let root = temp.path();
        fs::write(root.join("a.txt"), b"a").expect("write a");
        fs::write(root.join("b.txt"), b"b").expect("write b");

        let walk_data = WalkData::new(root, &[]);
        let mut data = crate::indexer::RootIndexData::from_walk(&walk_data).expect("walk");
        data.remove_entry(&root.join("a.txt"));

        let mut cloned = clone_slab_for_persistence(&data.file_nodes).expect("clone slab");

        // Corrupt the root chain by deleting one referenced child index.
        let root_id = data.file_nodes.root();
        let missing_child = cloned
            .get(root_id)
            .and_then(|node| node.children.first().copied())
            .expect("root should have a child");
        let _ = cloned.try_remove(missing_child);

        let name_index: BTreeMap<Box<str>, SortedSlabIndices> = data
            .name_index
            .iter()
            .map(|(name, indices)| ((*name).into(), indices.clone()))
            .collect();

        let storage = PersistentStorage {
            version: INDEX_CACHE_VERSION,
            last_event_id: 1,
            path: root.to_path_buf(),
            root_is_dir: true,
            ignore_paths: Vec::new(),
            slab_root: root_id,
            slab: cloned,
            name_index,
            rescan_count: 0,
            saved_at: 0,
            errors: 0,
        };

        assert!(
            !storage_graph_is_consistent(&storage),
            "validation should reject missing child references"
        );
    }
}
