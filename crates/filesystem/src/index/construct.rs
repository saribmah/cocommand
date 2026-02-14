//! Slab and name index construction from Node tree.
//!
//! This module implements Cardinal's approach of converting the `Node` tree
//! (built by fswalk) into a slab + name_index in a single recursive pass.
//!
//! Key insight: Because `fswalk` sorts children by name during walk,
//! the preorder traversal visits nodes in lexicographic path order.
//! This allows us to use `add_index_ordered()` which is O(1) per insertion
//! instead of binary search insertion which is O(log n).

use std::collections::BTreeMap;

use super::fswalk::Node;
use crate::namepool::NAME_POOL;
use crate::slab::{SlabIndex, SlabNode, SortedSlabIndices, ThinSlab};

/// Name index mapping filenames to sorted slab indices.
///
/// This matches Cardinal's `NameIndex` structure.
pub type NameIndex = BTreeMap<&'static str, SortedSlabIndices>;

/// Constructs the slab and name index from a Node tree.
///
/// Returns `(root_index, slab, name_index)`.
///
/// This function performs a preorder traversal of the node tree,
/// inserting each node into the slab and name index. Because children
/// are sorted by name, the preorder traversal produces indices in
/// lexicographic path order, allowing O(1) ordered insertion.
pub fn construct_slab_and_name_index(root: &Node) -> (SlabIndex, ThinSlab<SlabNode>, NameIndex) {
    let mut slab = ThinSlab::new();
    let mut name_index = NameIndex::new();

    let root_index = construct_node_recursive(None, root, &mut slab, &mut name_index);

    (root_index, slab, name_index)
}

/// Recursively constructs slab nodes from the Node tree.
///
/// This is the core of Cardinal's `construct_node_slab_name_index` function.
fn construct_node_recursive(
    parent: Option<SlabIndex>,
    node: &Node,
    slab: &mut ThinSlab<SlabNode>,
    name_index: &mut NameIndex,
) -> SlabIndex {
    // Intern the name in the global NAME_POOL
    let name = NAME_POOL.intern(&node.name);

    // Create metadata (use default if not available)
    let metadata = node.metadata.unwrap_or_default();

    // Create the slab node (children will be set after recursion)
    let slab_node = SlabNode::new(parent, name, metadata);
    let index = slab.insert(slab_node);

    // Add to name index using ordered insertion
    // SAFETY: fswalk sorts children by name, so preorder traversal
    // visits nodes in lexicographic path order.
    add_index_ordered(name_index, name, index);

    // Recursively process children and collect their indices
    let child_indices: Vec<SlabIndex> = node
        .children
        .iter()
        .map(|child| construct_node_recursive(Some(index), child, slab, name_index))
        .collect();

    // Set children on the node
    if let Some(slab_node) = slab.get_mut(index) {
        for child_index in child_indices {
            slab_node.add_child(child_index);
        }
    }

    index
}

/// Adds an index to the name index using ordered insertion.
///
/// This is O(1) because we rely on the preorder traversal visiting
/// nodes in lexicographic path order.
fn add_index_ordered(name_index: &mut NameIndex, name: &'static str, index: SlabIndex) {
    match name_index.get_mut(name) {
        Some(indices) => {
            // SAFETY: Preorder traversal ensures lexicographic order
            unsafe {
                indices.push_ordered(index);
            }
        }
        None => {
            name_index.insert(name, SortedSlabIndices::with_single(index));
        }
    }
}

/// Adds an index to the name index with proper sorting (for incremental updates).
///
/// This performs binary search insertion and is O(log n) per insertion.
/// Use this for single-entry updates after initial construction.
pub fn add_index_sorted<F>(
    name_index: &mut NameIndex,
    name: &'static str,
    index: SlabIndex,
    path_fn: F,
) where
    F: Fn(SlabIndex) -> Option<String>,
{
    match name_index.get_mut(name) {
        Some(indices) => {
            indices.insert_sorted(index, &path_fn);
        }
        None => {
            name_index.insert(name, SortedSlabIndices::with_single(index));
        }
    }
}

/// Removes an index from the name index.
pub fn remove_index(name_index: &mut NameIndex, name: &'static str, index: SlabIndex) -> bool {
    if let Some(indices) = name_index.get_mut(name) {
        let removed = indices.remove(index);
        if indices.is_empty() {
            name_index.remove(name);
        }
        removed
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slab::{NodeFileType, SlabNodeMetadata, StateTypeSize};

    fn make_metadata(is_dir: bool) -> SlabNodeMetadata {
        let file_type = if is_dir {
            NodeFileType::Dir
        } else {
            NodeFileType::File
        };
        SlabNodeMetadata {
            state_type_size: StateTypeSize::some(file_type, 100),
            ctime: 0,
            mtime: 0,
        }
    }

    #[test]
    fn construct_single_node() {
        let root = Node {
            children: vec![],
            name: "root".into(),
            metadata: Some(make_metadata(true)),
        };

        let (root_index, slab, name_index) = construct_slab_and_name_index(&root);

        assert_eq!(slab.len(), 1);
        assert_eq!(name_index.len(), 1);
        assert!(name_index.contains_key("root"));

        let node = slab.get(root_index).unwrap();
        assert_eq!(node.name(), "root");
        assert!(node.children.is_empty());
    }

    #[test]
    fn construct_with_children() {
        let root = Node {
            children: vec![
                Node {
                    children: vec![],
                    name: "alpha.txt".into(),
                    metadata: Some(make_metadata(false)),
                },
                Node {
                    children: vec![],
                    name: "beta.txt".into(),
                    metadata: Some(make_metadata(false)),
                },
            ],
            name: "root".into(),
            metadata: Some(make_metadata(true)),
        };

        let (root_index, slab, name_index) = construct_slab_and_name_index(&root);

        assert_eq!(slab.len(), 3);
        assert_eq!(name_index.len(), 3);

        let root_node = slab.get(root_index).unwrap();
        assert_eq!(root_node.children.len(), 2);

        // Verify children are correct
        let child1 = slab.get(root_node.children[0]).unwrap();
        let child2 = slab.get(root_node.children[1]).unwrap();
        assert_eq!(child1.name(), "alpha.txt");
        assert_eq!(child2.name(), "beta.txt");

        // Verify parent links
        assert_eq!(child1.parent(), Some(root_index));
        assert_eq!(child2.parent(), Some(root_index));
    }

    #[test]
    fn construct_nested_tree() {
        // /
        //   dir/
        //     subdir/
        //       file.txt
        let root = Node {
            children: vec![Node {
                children: vec![Node {
                    children: vec![Node {
                        children: vec![],
                        name: "file.txt".into(),
                        metadata: Some(make_metadata(false)),
                    }],
                    name: "subdir".into(),
                    metadata: Some(make_metadata(true)),
                }],
                name: "dir".into(),
                metadata: Some(make_metadata(true)),
            }],
            name: "/".into(),
            metadata: Some(make_metadata(true)),
        };

        let (root_index, slab, name_index) = construct_slab_and_name_index(&root);

        assert_eq!(slab.len(), 4);

        // Check name index has all names
        assert!(name_index.contains_key("/"));
        assert!(name_index.contains_key("dir"));
        assert!(name_index.contains_key("subdir"));
        assert!(name_index.contains_key("file.txt"));

        // Verify tree structure
        let root_node = slab.get(root_index).unwrap();
        assert_eq!(root_node.name(), "/");
        assert_eq!(root_node.children.len(), 1);

        let dir_node = slab.get(root_node.children[0]).unwrap();
        assert_eq!(dir_node.name(), "dir");
        assert_eq!(dir_node.parent(), Some(root_index));
    }

    #[test]
    fn duplicate_names_in_name_index() {
        // Multiple files with the same name in different directories
        let root = Node {
            children: vec![
                Node {
                    children: vec![Node {
                        children: vec![],
                        name: "file.txt".into(),
                        metadata: Some(make_metadata(false)),
                    }],
                    name: "a".into(),
                    metadata: Some(make_metadata(true)),
                },
                Node {
                    children: vec![Node {
                        children: vec![],
                        name: "file.txt".into(),
                        metadata: Some(make_metadata(false)),
                    }],
                    name: "b".into(),
                    metadata: Some(make_metadata(true)),
                },
            ],
            name: "root".into(),
            metadata: Some(make_metadata(true)),
        };

        let (_root_index, slab, name_index) = construct_slab_and_name_index(&root);

        assert_eq!(slab.len(), 5); // root, a, b, file.txt, file.txt

        // file.txt should have 2 entries in name_index
        let file_indices = name_index.get("file.txt").unwrap();
        assert_eq!(file_indices.len(), 2);
    }
}
