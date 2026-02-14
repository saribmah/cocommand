//! Core index data structures using memory-mapped slab storage.
//!
//! This module contains `RootIndexData` which holds the indexed filesystem entries
//! using Cardinal's two-structure approach:
//! 1. `FileNodes` - Hierarchical tree storage backed by `ThinSlab<SlabNode>`
//! 2. `NameIndex` - Filename to sorted slab indices mapping
//!
//! Unlike earlier implementations, we do NOT maintain:
//! - Path to node HashMaps (path lookups use tree traversal)
//! - Type indexes (file_ids, directory_ids)
//! - Extension index
//!
//! This matches Cardinal's minimalist approach where the slab + name_index
//! are sufficient for all operations.

use std::path::{Path, PathBuf};

use super::construct::{self, NameIndex};
use super::file_nodes::FileNodes;
use super::fswalk::{walk_it, WalkData};
use crate::namepool::NamePool;
use crate::slab::{SlabIndex, SlabNode, SlabNodeMetadata};

/// Runtime index data with slab-based node storage.
///
/// This struct combines `FileNodes` (the hierarchical tree storage) with
/// `NameIndex` (filename to indices mapping) for fast searches.
///
/// Matches Cardinal's `SearchCache` structure.
#[derive(Debug)]
pub struct RootIndexData {
    /// Hierarchical file node tree backed by slab storage.
    pub file_nodes: FileNodes,

    /// Maps filenames to sorted slab indices.
    /// Uses `BTreeMap` for deterministic iteration order.
    pub name_index: NameIndex,

    /// Error count during indexing.
    pub errors: usize,
}

impl Default for RootIndexData {
    fn default() -> Self {
        Self {
            file_nodes: FileNodes::empty(),
            name_index: NameIndex::new(),
            errors: 0,
        }
    }
}

impl RootIndexData {
    /// Creates a new empty RootIndexData.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds index data from a filesystem walk.
    ///
    /// This is the Cardinal-style two-phase approach:
    /// 1. Walk builds a Node tree (children sorted during walk)
    /// 2. Convert tree to slab + name_index in single recursive pass
    ///
    /// Returns `None` if the walk was cancelled.
    pub fn from_walk(walk_data: &WalkData) -> Option<Self> {
        // Phase 1: Walk the filesystem to build Node tree
        let node_tree = walk_it(walk_data)?;

        // Phase 2: Convert to slab + name_index
        let (root_index, slab, name_index) = construct::construct_slab_and_name_index(&node_tree);

        let file_nodes = FileNodes::new(slab, root_index);

        let errors = walk_data
            .num_files
            .load(std::sync::atomic::Ordering::Relaxed)
            + walk_data
                .num_dirs
                .load(std::sync::atomic::Ordering::Relaxed);
        // Note: We don't track errors during walk currently, so this is 0
        let _ = errors; // Suppress warning

        Some(Self {
            file_nodes,
            name_index,
            errors: 0,
        })
    }

    /// Gets a node by its index.
    #[inline]
    pub fn get_node(&self, id: SlabIndex) -> Option<&SlabNode> {
        self.file_nodes.get(id)
    }

    /// Iterates over all nodes.
    pub fn iter_nodes(&self) -> impl Iterator<Item = (SlabIndex, &SlabNode)> {
        self.file_nodes.iter()
    }

    /// Returns the number of indexed entries.
    pub fn len(&self) -> usize {
        self.file_nodes.len()
    }

    /// Returns true if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.file_nodes.is_empty()
    }

    /// Computes the full path for a node by walking up the parent chain.
    pub fn node_path(&self, id: SlabIndex) -> Option<PathBuf> {
        self.file_nodes.node_path(id)
    }

    /// Locates the slab index for an absolute path by tree traversal.
    pub fn node_index_for_path(&self, path: &Path) -> Option<SlabIndex> {
        self.file_nodes.node_index_for_path(path)
    }

    // -------------------------------------------------------------------------
    // Compatibility methods for search (computed on-demand)
    // -------------------------------------------------------------------------

    /// Gets the node index for a path (case-sensitive or insensitive lookup).
    ///
    /// For case-sensitive, uses tree traversal.
    /// For case-insensitive, searches through all nodes (slower but correct).
    pub fn node_id_for_path(&self, path: &str, case_sensitive: bool) -> Option<SlabIndex> {
        if case_sensitive {
            // Use tree traversal for case-sensitive
            self.node_index_for_path(std::path::Path::new(path))
        } else {
            // For case-insensitive, need to search through nodes
            // This is slower but correct - Cardinal also doesn't have a fast path for this
            let lower_path = path.to_ascii_lowercase();
            for (idx, _node) in self.iter_nodes() {
                if let Some(node_path) = self.node_path(idx) {
                    if node_path.to_string_lossy().to_ascii_lowercase() == lower_path {
                        return Some(idx);
                    }
                }
            }
            None
        }
    }

    /// Returns all file node indices (computed by iterating).
    pub fn file_ids(&self) -> std::collections::BTreeSet<SlabIndex> {
        self.iter_nodes()
            .filter(|(_, node)| node.is_file())
            .map(|(idx, _)| idx)
            .collect()
    }

    /// Returns all directory node indices (computed by iterating).
    pub fn directory_ids(&self) -> std::collections::BTreeSet<SlabIndex> {
        self.iter_nodes()
            .filter(|(_, node)| node.is_dir())
            .map(|(idx, _)| idx)
            .collect()
    }

    /// Gets indices for files with a given extension (computed by iterating).
    pub fn indices_for_extension(&self, extension: &str) -> std::collections::BTreeSet<SlabIndex> {
        let lower_ext = extension.to_ascii_lowercase();
        self.iter_nodes()
            .filter(|(_, node)| {
                node.is_file()
                    && node.extension().map(|e| e.to_ascii_lowercase()) == Some(lower_ext.clone())
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    /// Gets all slab indices for entries with the given filename.
    ///
    /// Returns `None` if no entries have this name.
    #[allow(dead_code)] // Used by tests
    pub fn indices_for_name(&self, name: &str) -> Option<&crate::slab::SortedSlabIndices> {
        self.name_index.get(name)
    }

    // -------------------------------------------------------------------------
    // Incremental update methods (for file watcher events)
    // -------------------------------------------------------------------------

    /// Inserts or updates an entry at the given path.
    pub fn upsert_entry(&mut self, path: &Path, name_pool: &NamePool) {
        // Remove existing entry if present
        if let Some(existing_id) = self.node_index_for_path(path) {
            self.remove_node(existing_id);
        }

        // Get parent path and find/create parent node
        let Some(parent_path) = path.parent() else {
            return;
        };

        let parent_id = self.node_index_for_path(parent_path);

        // Get metadata for the path
        let metadata = std::fs::symlink_metadata(path).ok();
        let node_metadata = metadata
            .as_ref()
            .map(SlabNodeMetadata::from_fs_metadata)
            .unwrap_or_default();

        // Get the filename
        let Some(file_name) = path.file_name() else {
            return;
        };
        let name = name_pool.intern(&file_name.to_string_lossy());

        // Create the node
        let node = SlabNode::new(parent_id, name, node_metadata);
        let id = self.file_nodes.insert(node);

        // Add to parent's children
        if let Some(parent_id) = parent_id {
            if let Some(parent_node) = self.file_nodes.get_mut(parent_id) {
                parent_node.add_child(id);
            }
        }

        // Add to name index (with proper sorting)
        let file_nodes = &self.file_nodes;
        construct::add_index_sorted(&mut self.name_index, name, id, |idx| {
            file_nodes
                .node_path(idx)
                .map(|p| p.to_string_lossy().to_string())
        });
    }

    /// Removes a node and all its descendants.
    pub fn remove_entry(&mut self, path: &Path) {
        let Some(id) = self.node_index_for_path(path) else {
            return;
        };
        self.remove_node(id);
    }

    /// Removes a node by its index (and updates indexes).
    fn remove_node(&mut self, id: SlabIndex) {
        let Some(node) = self.file_nodes.get(id) else {
            return;
        };

        // Get node info before removal
        let parent_id = node.parent();
        let name = node.name();
        let children: Vec<_> = node.children.iter().copied().collect();

        // Remove from parent's children
        if let Some(parent_id) = parent_id {
            if let Some(parent_node) = self.file_nodes.get_mut(parent_id) {
                parent_node.remove_child(id);
            }
        }

        // Remove from name index
        construct::remove_index(&mut self.name_index, name, id);

        // Recursively remove children
        for child_id in children {
            self.remove_node(child_id);
        }

        // Remove from slab
        self.file_nodes.try_remove(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::sync::atomic::AtomicBool;
    use tempfile::TempDir;

    #[test]
    fn from_walk_empty_directory() {
        let temp = TempDir::new().unwrap();
        let walk_data = WalkData::new(temp.path(), &[]);
        let data = RootIndexData::from_walk(&walk_data);

        assert!(data.is_some());
        let data = data.unwrap();
        assert!(!data.is_empty());
    }

    #[test]
    fn from_walk_with_files() {
        let temp = TempDir::new().unwrap();
        File::create(temp.path().join("a.txt")).unwrap();
        File::create(temp.path().join("b.txt")).unwrap();

        let walk_data = WalkData::new(temp.path(), &[]);
        let data = RootIndexData::from_walk(&walk_data).unwrap();

        // Should have entries in name index
        assert!(data.name_index.contains_key("a.txt"));
        assert!(data.name_index.contains_key("b.txt"));
    }

    #[test]
    fn from_walk_cancellation() {
        let temp = TempDir::new().unwrap();
        let cancel = AtomicBool::new(true);
        let walk_data = WalkData::new(temp.path(), &[]).with_cancel(&cancel);
        let data = RootIndexData::from_walk(&walk_data);

        assert!(data.is_none());
    }

    #[test]
    fn node_path_reconstruction() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("subdir")).unwrap();
        File::create(temp.path().join("subdir/file.txt")).unwrap();

        let walk_data = WalkData::new(temp.path(), &[]);
        let data = RootIndexData::from_walk(&walk_data).unwrap();

        // Find the file node
        let file_indices = data.indices_for_name("file.txt").unwrap();
        assert_eq!(file_indices.len(), 1);

        let file_id = file_indices.iter().next().copied().unwrap();
        let path = data.node_path(file_id).unwrap();

        // Path should end with subdir/file.txt
        assert!(path.to_string_lossy().ends_with("subdir/file.txt"));
    }

    #[test]
    fn duplicate_filenames() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("a")).unwrap();
        fs::create_dir(temp.path().join("b")).unwrap();
        File::create(temp.path().join("a/same.txt")).unwrap();
        File::create(temp.path().join("b/same.txt")).unwrap();

        let walk_data = WalkData::new(temp.path(), &[]);
        let data = RootIndexData::from_walk(&walk_data).unwrap();

        // Should have 2 entries for same.txt
        let indices = data.indices_for_name("same.txt").unwrap();
        assert_eq!(indices.len(), 2);
    }
}
