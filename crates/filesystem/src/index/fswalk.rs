//! Filesystem walking that builds a tree structure.
//!
//! This module implements Cardinal's `fswalk` approach:
//! - Walk builds a `Node` tree (not a flat list)
//! - Children are sorted by name during walk (not after)
//! - Result is wrapped in parent chain back to `/`
//!
//! This enables efficient slab construction via preorder traversal,
//! where nodes are visited in lexicographic path order.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rayon::prelude::*;

use super::build::IndexBuildProgress;
use crate::slab::SlabNodeMetadata;

/// A node in the filesystem tree built during walking.
///
/// This is the intermediate representation between filesystem walking
/// and slab construction. Children are sorted by name to ensure
/// preorder traversal produces lexicographic path ordering.
#[derive(Debug)]
pub struct Node {
    /// Child nodes (sorted by name after collection).
    pub children: Vec<Node>,
    /// The filename (not the full path).
    pub name: Box<str>,
    /// File metadata (type, size, times).
    pub metadata: Option<SlabNodeMetadata>,
}

impl Node {
    /// Creates a new node with the given name and optional metadata.
    pub fn new(name: Box<str>, metadata: Option<SlabNodeMetadata>) -> Self {
        Self {
            children: Vec::new(),
            name,
            metadata,
        }
    }

    /// Creates a leaf node (file) with metadata.
    pub fn leaf(name: Box<str>, metadata: SlabNodeMetadata) -> Self {
        Self {
            children: Vec::new(),
            name,
            metadata: Some(metadata),
        }
    }
}

/// Data passed to the parallel walker.
#[derive(Debug)]
pub struct WalkData<'a> {
    /// Number of files scanned (atomic counter).
    pub num_files: AtomicUsize,
    /// Number of directories scanned (atomic counter).
    pub num_dirs: AtomicUsize,
    /// Cancellation flag (checked periodically).
    pub cancel: Option<&'a AtomicBool>,
    /// Root path being indexed.
    pub root_path: &'a Path,
    /// Paths to ignore during walking.
    pub ignore_directories: &'a [PathBuf],
    /// Optional progress tracker for UI updates.
    pub progress: Option<&'a IndexBuildProgress>,
}

impl<'a> WalkData<'a> {
    /// Creates new walk data with the given root path.
    pub fn new(root_path: &'a Path, ignore_directories: &'a [PathBuf]) -> Self {
        Self {
            num_files: AtomicUsize::new(0),
            num_dirs: AtomicUsize::new(0),
            cancel: None,
            root_path,
            ignore_directories,
            progress: None,
        }
    }

    /// Sets the cancellation flag.
    pub fn with_cancel(mut self, cancel: &'a AtomicBool) -> Self {
        self.cancel = Some(cancel);
        self
    }

    /// Sets the progress tracker.
    pub fn with_progress(mut self, progress: &'a IndexBuildProgress) -> Self {
        self.progress = Some(progress);
        self
    }

    /// Returns true if the given path should be ignored.
    fn should_ignore(&self, path: &Path) -> bool {
        self.ignore_directories
            .iter()
            .any(|ignored| path == ignored || path.starts_with(ignored))
    }

    /// Returns true if cancellation was requested.
    fn is_cancelled(&self) -> bool {
        self.cancel
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(false)
    }
}

/// Walks the filesystem and builds a node tree.
///
/// Returns `None` if cancelled, otherwise returns the root node.
/// The result is wrapped in a parent chain back to `/` so the tree
/// represents the full path from filesystem root.
pub fn walk_it(walk_data: &WalkData) -> Option<Node> {
    walk(walk_data.root_path, walk_data).map(|node_tree| {
        // Wrap the result in parent chain back to /
        if let Some(parent) = walk_data.root_path.parent() {
            let mut path = PathBuf::from(parent);
            let mut node = Node {
                children: vec![node_tree],
                name: path
                    .iter()
                    .next_back()
                    .expect("at least one parent segment in root path")
                    .to_string_lossy()
                    .into_owned()
                    .into_boxed_str(),
                metadata: metadata_of_path(&path),
            };
            while path.pop() {
                // Check if we've reached the root "/"
                if path.as_os_str() == "/" {
                    // Wrap in root "/" node and return
                    return Node {
                        children: vec![node],
                        name: "/".into(),
                        metadata: None,
                    };
                }
                if path.as_os_str().is_empty() {
                    break;
                }
                node = Node {
                    children: vec![node],
                    name: path
                        .iter()
                        .next_back()
                        .map(|s| s.to_string_lossy().into_owned().into_boxed_str())
                        .unwrap_or_else(|| "".into()),
                    metadata: metadata_of_path(&path),
                };
            }
            // Fallback: wrap in root "/" node
            Node {
                children: vec![node],
                name: "/".into(),
                metadata: None,
            }
        } else {
            // Root path is "/" itself
            node_tree
        }
    })
}

/// Core recursive walk function.
///
/// Walks a directory tree in parallel using rayon, building a `Node` tree.
/// Children are sorted by name after collection to ensure deterministic
/// preorder traversal order.
fn walk(path: &Path, walk_data: &WalkData) -> Option<Node> {
    // Check cancellation
    if walk_data.is_cancelled() {
        return None;
    }

    // Check if path should be ignored
    if walk_data.should_ignore(path) {
        return None;
    }

    // Get metadata
    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => {
            // Can't access path, skip it
            return None;
        }
    };

    let file_type = metadata.file_type();
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned().into_boxed_str())
        .unwrap_or_else(|| {
            // Handle root paths like "/" or "C:\"
            path.to_string_lossy().into_owned().into_boxed_str()
        });

    // Handle directories
    if file_type.is_dir() {
        walk_data.num_dirs.fetch_add(1, Ordering::Relaxed);
        if let Some(progress) = walk_data.progress {
            progress.scanned_dirs.fetch_add(1, Ordering::Relaxed);
        }

        let read_dir = match fs::read_dir(path) {
            Ok(iter) => iter,
            Err(_) => {
                // Can't read directory, return it without children
                return Some(Node::new(
                    name,
                    Some(SlabNodeMetadata::from_fs_metadata(&metadata)),
                ));
            }
        };

        // Collect directory entries
        let entries: Vec<_> = read_dir.filter_map(Result::ok).collect();

        // Walk children in parallel
        let mut children: Vec<Node> = entries
            .into_par_iter()
            .filter_map(|entry| {
                // Check cancellation in parallel iteration
                if walk_data.is_cancelled() {
                    return None;
                }

                let child_path = entry.path();

                // Check if should be ignored
                if walk_data.should_ignore(&child_path) {
                    return None;
                }

                // Get file type without following symlinks
                let Ok(file_type) = entry.file_type() else {
                    return None;
                };

                if file_type.is_dir() {
                    // Recurse into directory
                    walk(&child_path, walk_data)
                } else {
                    // File or symlink - create leaf node
                    walk_data.num_files.fetch_add(1, Ordering::Relaxed);
                    if let Some(progress) = walk_data.progress {
                        progress.scanned_files.fetch_add(1, Ordering::Relaxed);
                    }

                    let child_name = entry
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                        .into_boxed_str();

                    let child_metadata = entry
                        .metadata()
                        .ok()
                        .map(|m| SlabNodeMetadata::from_fs_metadata(&m));

                    Some(Node {
                        children: Vec::new(),
                        name: child_name,
                        metadata: child_metadata,
                    })
                }
            })
            .collect();

        // Check cancellation after parallel collection
        if walk_data.is_cancelled() {
            return None;
        }

        // CRITICAL: Sort children by name for deterministic preorder traversal
        children.sort_unstable_by(|a, b| a.name.cmp(&b.name));

        Some(Node {
            children,
            name,
            metadata: Some(SlabNodeMetadata::from_fs_metadata(&metadata)),
        })
    } else {
        // File or symlink
        walk_data.num_files.fetch_add(1, Ordering::Relaxed);
        if let Some(progress) = walk_data.progress {
            progress.scanned_files.fetch_add(1, Ordering::Relaxed);
        }

        Some(Node::leaf(
            name,
            SlabNodeMetadata::from_fs_metadata(&metadata),
        ))
    }
}

/// Gets metadata for a path, returning None if inaccessible.
fn metadata_of_path(path: &Path) -> Option<SlabNodeMetadata> {
    fs::symlink_metadata(path)
        .ok()
        .map(|m| SlabNodeMetadata::from_fs_metadata(&m))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    #[test]
    fn walk_empty_directory() {
        let temp = TempDir::new().unwrap();
        let walk_data = WalkData::new(temp.path(), &[]);
        let result = walk_it(&walk_data);

        assert!(result.is_some());
        let root = result.unwrap();
        // Should have the temp dir as a descendant
        assert!(!root.children.is_empty() || root.name.as_ref() == "/");
    }

    #[test]
    fn walk_with_files() {
        let temp = TempDir::new().unwrap();
        File::create(temp.path().join("a.txt")).unwrap();
        File::create(temp.path().join("b.txt")).unwrap();
        File::create(temp.path().join("c.txt")).unwrap();

        let walk_data = WalkData::new(temp.path(), &[]);
        let result = walk(temp.path(), &walk_data);

        assert!(result.is_some());
        let node = result.unwrap();
        assert_eq!(node.children.len(), 3);

        // Children should be sorted by name
        assert_eq!(node.children[0].name.as_ref(), "a.txt");
        assert_eq!(node.children[1].name.as_ref(), "b.txt");
        assert_eq!(node.children[2].name.as_ref(), "c.txt");
    }

    #[test]
    fn walk_with_subdirs() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("aaa")).unwrap();
        fs::create_dir(temp.path().join("bbb")).unwrap();
        File::create(temp.path().join("aaa/file.txt")).unwrap();

        let walk_data = WalkData::new(temp.path(), &[]);
        let result = walk(temp.path(), &walk_data);

        assert!(result.is_some());
        let node = result.unwrap();

        // Should have 2 children (both directories)
        assert_eq!(node.children.len(), 2);
        assert_eq!(node.children[0].name.as_ref(), "aaa");
        assert_eq!(node.children[1].name.as_ref(), "bbb");

        // aaa should have one child
        assert_eq!(node.children[0].children.len(), 1);
        assert_eq!(node.children[0].children[0].name.as_ref(), "file.txt");
    }

    #[test]
    fn walk_ignores_paths() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("include")).unwrap();
        fs::create_dir(temp.path().join("exclude")).unwrap();
        File::create(temp.path().join("include/a.txt")).unwrap();
        File::create(temp.path().join("exclude/b.txt")).unwrap();

        let ignore = vec![temp.path().join("exclude")];
        let walk_data = WalkData::new(temp.path(), &ignore);
        let result = walk(temp.path(), &walk_data);

        assert!(result.is_some());
        let node = result.unwrap();

        // Should only have include directory
        assert_eq!(node.children.len(), 1);
        assert_eq!(node.children[0].name.as_ref(), "include");
    }

    #[test]
    fn walk_cancellation() {
        let temp = TempDir::new().unwrap();
        File::create(temp.path().join("file.txt")).unwrap();

        let cancel = AtomicBool::new(true); // Pre-cancelled
        let walk_data = WalkData::new(temp.path(), &[]).with_cancel(&cancel);
        let result = walk(temp.path(), &walk_data);

        // Should return None due to cancellation
        assert!(result.is_none());
    }

    #[test]
    fn children_sorted_alphabetically() {
        let temp = TempDir::new().unwrap();
        // Create files in non-alphabetical order
        File::create(temp.path().join("zebra.txt")).unwrap();
        File::create(temp.path().join("apple.txt")).unwrap();
        File::create(temp.path().join("mango.txt")).unwrap();

        let walk_data = WalkData::new(temp.path(), &[]);
        let result = walk(temp.path(), &walk_data);

        let node = result.unwrap();
        let names: Vec<_> = node.children.iter().map(|c| c.name.as_ref()).collect();

        // Should be sorted
        assert_eq!(names, vec!["apple.txt", "mango.txt", "zebra.txt"]);
    }
}
