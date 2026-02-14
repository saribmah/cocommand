//! Node view helpers for computing derived properties from slab nodes.
//!
//! This module provides utilities to compute paths, depths, and other
//! derived properties from the compact slab node storage without
//! storing redundant data.

use crate::slab::{SlabIndex, SlabNode, ThinSlab};

/// A view into a node that can compute derived properties.
///
/// This struct holds a reference to the slab and a node index, allowing
/// efficient computation of paths, depths, and other properties by
/// traversing the parent chain.
pub struct NodeView<'a> {
    slab: &'a ThinSlab<SlabNode>,
    index: SlabIndex,
}

impl<'a> NodeView<'a> {
    /// Creates a new node view.
    #[inline]
    pub fn new(slab: &'a ThinSlab<SlabNode>, index: SlabIndex) -> Self {
        Self { slab, index }
    }

    /// Computes the full path by walking up the parent chain.
    ///
    /// Returns the absolute path built from all segments (starting from `/`).
    /// The `_root_path` parameter is unused but kept for API compatibility.
    pub fn compute_path(&self, _root_path: &str) -> Option<String> {
        let mut segments = Vec::new();
        let mut current = self.index;

        loop {
            let node = self.slab.get(current)?;
            segments.push(node.name());

            match node.parent() {
                Some(parent) => current = parent,
                None => break,
            }
        }

        // Reverse to get root-to-leaf order
        segments.reverse();

        // Build absolute path string starting from "/"
        // The first segment should be "/" (root), so we skip it and start path with "/"
        if segments.is_empty() {
            return Some("/".to_string());
        }

        let mut path = String::new();
        for segment in &segments {
            if *segment == "/" {
                continue; // Skip the "/" root node
            }
            path.push('/');
            path.push_str(segment);
        }

        // If path is empty (only had "/" root), return "/"
        if path.is_empty() {
            return Some("/".to_string());
        }

        Some(path)
    }

    /// Computes the depth (number of ancestors, 0 for root nodes).
    pub fn compute_depth(&self) -> Option<usize> {
        let mut depth = 0;
        let mut current = self.index;

        loop {
            let node = self.slab.get(current)?;
            match node.parent() {
                Some(parent) => {
                    depth += 1;
                    current = parent;
                }
                None => break,
            }
        }

        Some(depth)
    }

    /// Returns true if this node or any ancestor within `max_depth` levels is hidden.
    ///
    /// This only checks `max_depth` ancestors, not all the way to the root.
    /// Useful when the indexed root itself may be hidden but we want to search inside it.
    pub fn is_hidden_within_depth(&self, max_depth: usize) -> Option<bool> {
        let mut current = self.index;
        let mut depth = 0;

        loop {
            if depth > max_depth {
                break;
            }
            let node = self.slab.get(current)?;
            if node.is_hidden() {
                return Some(true);
            }
            match node.parent() {
                Some(parent) => {
                    current = parent;
                    depth += 1;
                }
                None => break,
            }
        }

        Some(false)
    }

    /// Returns true if this node or any ancestor is hidden (starts with '.').
    ///
    /// Walks the entire parent chain to the root.
    #[allow(dead_code)] // Used by tests
    pub fn is_hidden_recursive(&self) -> Option<bool> {
        let mut current = self.index;

        loop {
            let node = self.slab.get(current)?;
            if node.is_hidden() {
                return Some(true);
            }
            match node.parent() {
                Some(parent) => current = parent,
                None => break,
            }
        }

        Some(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slab::{NodeFileType, SlabNodeMetadata, StateTypeSize};

    fn make_node(name: &'static str, parent: Option<SlabIndex>) -> SlabNode {
        let metadata = SlabNodeMetadata {
            state_type_size: StateTypeSize::some(NodeFileType::File, 100),
            ctime: 0,
            mtime: 1700000000,
        };
        SlabNode::new(parent, name, metadata)
    }

    fn make_dir_node(name: &'static str, parent: Option<SlabIndex>) -> SlabNode {
        let metadata = SlabNodeMetadata {
            state_type_size: StateTypeSize::some(NodeFileType::Dir, 0),
            ctime: 0,
            mtime: 1700000000,
        };
        SlabNode::new(parent, name, metadata)
    }

    #[test]
    fn test_compute_path_single_node() {
        let mut slab = ThinSlab::new();
        // Create a proper tree with "/" as root
        let slash_idx = slab.insert(make_dir_node("/", None));
        let root_idx = slab.insert(make_dir_node("root", Some(slash_idx)));
        slab.get_mut(slash_idx).unwrap().add_child(root_idx);

        let view = NodeView::new(&slab, root_idx);
        let path = view.compute_path("ignored");
        assert_eq!(path, Some("/root".to_string()));
    }

    #[test]
    fn test_compute_path_nested() {
        let mut slab = ThinSlab::new();
        // Create a proper tree with "/" as root
        let slash_idx = slab.insert(make_dir_node("/", None));
        let src_idx = slab.insert(make_dir_node("src", Some(slash_idx)));
        let lib_idx = slab.insert(make_dir_node("lib", Some(src_idx)));
        let file_idx = slab.insert(make_node("mod.rs", Some(lib_idx)));

        // Update children
        slab.get_mut(slash_idx).unwrap().add_child(src_idx);
        slab.get_mut(src_idx).unwrap().add_child(lib_idx);
        slab.get_mut(lib_idx).unwrap().add_child(file_idx);

        let view = NodeView::new(&slab, file_idx);
        let path = view.compute_path("ignored");
        assert_eq!(path, Some("/src/lib/mod.rs".to_string()));
    }

    #[test]
    fn test_compute_depth() {
        let mut slab = ThinSlab::new();
        let root_idx = slab.insert(make_dir_node("a", None));
        let b_idx = slab.insert(make_dir_node("b", Some(root_idx)));
        let c_idx = slab.insert(make_node("c.txt", Some(b_idx)));

        assert_eq!(NodeView::new(&slab, root_idx).compute_depth(), Some(0));
        assert_eq!(NodeView::new(&slab, b_idx).compute_depth(), Some(1));
        assert_eq!(NodeView::new(&slab, c_idx).compute_depth(), Some(2));
    }

    #[test]
    fn test_is_hidden_recursive() {
        let mut slab = ThinSlab::new();
        let visible_dir = slab.insert(make_dir_node("visible", None));
        let hidden_dir = slab.insert(make_dir_node(".hidden", Some(visible_dir)));
        let file_in_hidden = slab.insert(make_node("file.txt", Some(hidden_dir)));

        assert_eq!(
            NodeView::new(&slab, visible_dir).is_hidden_recursive(),
            Some(false)
        );
        assert_eq!(
            NodeView::new(&slab, hidden_dir).is_hidden_recursive(),
            Some(true)
        );
        assert_eq!(
            NodeView::new(&slab, file_in_hidden).is_hidden_recursive(),
            Some(true)
        );
    }
}
