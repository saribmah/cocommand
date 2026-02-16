//! FileNodes - hierarchical tree wrapper for slab-based node storage.
//!
//! This module provides `FileNodes` which encapsulates the slab storage along with
//! the root node index.

use std::ffi::OsStr;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use crate::storage::{SlabIndex, SlabNode, ThinSlab};

/// Hierarchical file node tree backed by slab storage.
///
/// This struct combines the slab storage with the root node index,
/// enabling tree traversal operations like `node_path()` and `node_index_for_path()`.
#[derive(Debug)]
pub struct FileNodes {
    /// Memory-mapped slab storage for nodes.
    slab: ThinSlab<SlabNode>,
    /// Index of the root node in the slab.
    root: SlabIndex,
}

impl FileNodes {
    /// Creates a new FileNodes instance.
    pub fn new(slab: ThinSlab<SlabNode>, root: SlabIndex) -> Self {
        Self { slab, root }
    }

    /// Creates an empty FileNodes.
    pub fn empty() -> Self {
        Self {
            slab: ThinSlab::new(),
            root: SlabIndex::INVALID,
        }
    }

    /// Returns the root node index.
    #[inline]
    pub fn root(&self) -> SlabIndex {
        self.root
    }

    /// Returns true if the root index is valid.
    #[inline]
    pub fn has_root(&self) -> bool {
        self.root != SlabIndex::INVALID
    }

    /// Computes the full path for a node by walking up the parent chain.
    ///
    /// Returns `None` if the node doesn't exist or the chain is broken.
    pub fn node_path(&self, index: SlabIndex) -> Option<PathBuf> {
        let mut current = index;
        let mut segments = Vec::new();

        while let Some(node) = self.slab.get(current) {
            segments.push(node.name());
            match node.parent() {
                Some(parent) => current = parent,
                None => break,
            }
        }

        if segments.is_empty() {
            return None;
        }

        // Build path from root to node
        Some(
            std::iter::once("/")
                .chain(segments.into_iter().rev())
                .map(OsStr::new)
                .collect(),
        )
    }

    /// Locates the slab index for an absolute path when it belongs to the watch root.
    ///
    /// Traverses the tree from root, matching path segments to node names.
    pub fn node_index_for_path(&self, path: &Path) -> Option<SlabIndex> {
        let Ok(path) = path.strip_prefix("/") else {
            return None;
        };

        if !self.has_root() {
            return None;
        }

        let mut current = self.root;
        for segment in path {
            let next = self.slab.get(current)?.children.iter().find_map(|&child| {
                let name = self.slab.get(child)?.name();
                if OsStr::new(name) == segment {
                    Some(child)
                } else {
                    None
                }
            })?;
            current = next;
        }
        Some(current)
    }

    /// Returns the number of nodes in the slab.
    #[inline]
    pub fn len(&self) -> usize {
        self.slab.len()
    }

    /// Returns true if the slab is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.slab.is_empty()
    }

    /// Returns all subnodes (children, grandchildren, etc.) of a node.
    ///
    /// Used for collecting all affected nodes during tree operations.
    #[allow(dead_code)] // Used by tests
    pub fn all_subnodes(&self, index: SlabIndex) -> Vec<SlabIndex> {
        let mut result = Vec::new();
        let mut stack = Vec::new();

        // Start with direct children
        if let Some(node) = self.slab.get(index) {
            stack.extend(node.children.iter().copied());
        }

        while let Some(current) = stack.pop() {
            result.push(current);
            if let Some(node) = self.slab.get(current) {
                stack.extend(node.children.iter().copied());
            }
        }

        result
    }
}

impl Deref for FileNodes {
    type Target = ThinSlab<SlabNode>;

    fn deref(&self) -> &Self::Target {
        &self.slab
    }
}

impl DerefMut for FileNodes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.slab
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{NodeFileType, SlabNodeMetadata, StateTypeSize};

    fn make_test_node(parent: Option<SlabIndex>, name: &'static str, is_dir: bool) -> SlabNode {
        let file_type = if is_dir {
            NodeFileType::Dir
        } else {
            NodeFileType::File
        };
        SlabNode::new(
            parent,
            name,
            SlabNodeMetadata {
                state_type_size: StateTypeSize::some(file_type, 0),
                ctime: 0,
                mtime: 0,
            },
        )
    }

    #[test]
    fn test_file_nodes_basic() {
        let mut slab = ThinSlab::new();

        // Create root node "/"
        let root_idx = slab.insert(make_test_node(None, "", true));

        // Create "src" directory under root
        let src_idx = slab.insert(make_test_node(Some(root_idx), "src", true));
        slab.get_mut(root_idx).unwrap().add_child(src_idx);

        // Create "main.rs" file under src
        let main_idx = slab.insert(make_test_node(Some(src_idx), "main.rs", false));
        slab.get_mut(src_idx).unwrap().add_child(main_idx);

        let file_nodes = FileNodes::new(slab, root_idx);

        assert!(file_nodes.has_root());
        assert_eq!(file_nodes.root(), root_idx);
        assert_eq!(file_nodes.len(), 3);

        // Test node_path
        let path = file_nodes.node_path(main_idx);
        assert!(path.is_some());
        // The path should be /src/main.rs (relative to the slab's internal structure)
        let path_str = path.unwrap();
        assert!(path_str.to_string_lossy().contains("main.rs"));
    }

    #[test]
    fn test_file_nodes_empty() {
        let file_nodes = FileNodes::empty();
        assert!(!file_nodes.has_root());
        assert!(file_nodes.is_empty());
        assert_eq!(
            file_nodes.node_index_for_path(Path::new("/test/file.txt")),
            None
        );
    }

    #[test]
    fn test_all_subnodes() {
        let mut slab = ThinSlab::new();

        let root_idx = slab.insert(make_test_node(None, "", true));
        let child1_idx = slab.insert(make_test_node(Some(root_idx), "a", true));
        let child2_idx = slab.insert(make_test_node(Some(root_idx), "b", false));
        let grandchild_idx = slab.insert(make_test_node(Some(child1_idx), "c", false));

        slab.get_mut(root_idx).unwrap().add_child(child1_idx);
        slab.get_mut(root_idx).unwrap().add_child(child2_idx);
        slab.get_mut(child1_idx).unwrap().add_child(grandchild_idx);

        let file_nodes = FileNodes::new(slab, root_idx);

        let subnodes = file_nodes.all_subnodes(root_idx);
        assert_eq!(subnodes.len(), 3);
        assert!(subnodes.contains(&child1_idx));
        assert!(subnodes.contains(&child2_idx));
        assert!(subnodes.contains(&grandchild_idx));
    }
}
