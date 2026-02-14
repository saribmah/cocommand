//! Filesystem path utilities for watching.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Checks if a path is within the index scope.
pub fn path_in_scope(root: &Path, root_is_dir: bool, candidate: &Path) -> bool {
    if root_is_dir {
        candidate == root || candidate.starts_with(root)
    } else {
        candidate == root
    }
}

/// Checks if a path should be ignored.
pub fn path_is_ignored(ignored_roots: &[PathBuf], candidate: &Path) -> bool {
    ignored_roots
        .iter()
        .any(|ignored| candidate == ignored || candidate.starts_with(ignored))
}

/// Computes the minimal set of paths that must be rescanned for a batch of filesystem events.
///
/// This function implements Cardinal's efficient path coalescing algorithm:
///
/// 1. Sort paths by depth (shallowest first), then lexicographically
/// 2. Use a HashSet for O(1) ancestor lookup
/// 3. For each path, walk up the parent chain to check if any ancestor is already selected
/// 4. Skip paths that are covered by an ancestor
///
/// ## Goals
///
/// - Skip a path if it is already covered by an ancestor (`path.starts_with(ancestor)`)
/// - Keep only a single entry for identical paths
/// - Return the minimal cover—the smallest set of paths whose rescans cover every change
///
/// ## Complexity
///
/// O(n log n + m * depth): sort by depth first, then scan linearly while checking ancestors.
/// This is more efficient than the naive O(n²) approach of checking `starts_with()` against
/// all previously selected paths.
///
/// ## Examples
///
/// ```text
/// Input:  ["/a/b/c", "/a/b", "/a/b/d", "/x/y"]
/// Output: ["/a/b", "/x/y"]  (children "/a/b/c" and "/a/b/d" are covered by "/a/b")
/// ```
pub fn coalesce_event_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    if paths.len() <= 1 {
        return paths;
    }

    // Pair each path with its depth for sorting
    let mut candidates: Vec<(PathBuf, usize)> = paths
        .into_iter()
        .map(|path| {
            let depth = path_depth(&path);
            (path, depth)
        })
        .collect();

    // Sort by depth first (shallowest ancestors come first), then by path for determinism
    candidates.sort_unstable_by(|(path_a, depth_a), (path_b, depth_b)| {
        depth_a.cmp(depth_b).then_with(|| path_a.cmp(path_b))
    });

    // Deduplicate identical paths (after sorting, duplicates are adjacent)
    candidates.dedup_by(|(path_a, _), (path_b, _)| path_a == path_b);

    // Build the minimal cover using a HashSet for fast ancestor lookups
    let mut selected = Vec::with_capacity(candidates.len());
    let mut selected_set = HashSet::with_capacity(candidates.len());

    for (path, _depth) in candidates {
        // Skip if any ancestor is already selected
        if has_selected_ancestor(&path, &selected_set) {
            continue;
        }
        selected_set.insert(path.clone());
        selected.push(path);
    }

    selected
}

/// Returns the depth (number of path components) of a path.
#[inline]
fn path_depth(path: &Path) -> usize {
    path.components().count()
}

/// Checks if any ancestor of `path` is in the selected set.
///
/// Walks up the parent chain from the path to the root, checking each ancestor
/// against the selected set. This is O(depth) per path, but avoids the O(n)
/// linear scan of checking `starts_with()` against all selected paths.
fn has_selected_ancestor(path: &Path, selected: &HashSet<PathBuf>) -> bool {
    if selected.is_empty() {
        return false;
    }

    // Check if the exact path is already selected
    if selected.contains(path) {
        return true;
    }

    // Walk up the parent chain
    let mut ancestor = path.to_path_buf();
    while ancestor.pop() {
        if selected.contains(&ancestor) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesce_empty_input() {
        let paths: Vec<PathBuf> = vec![];
        let result = coalesce_event_paths(paths);
        assert!(result.is_empty());
    }

    #[test]
    fn coalesce_single_path() {
        let paths = vec![PathBuf::from("/a/b/c")];
        let result = coalesce_event_paths(paths);
        assert_eq!(result, vec![PathBuf::from("/a/b/c")]);
    }

    #[test]
    fn coalesce_removes_children_of_ancestor() {
        // When an ancestor is present, children should be filtered out
        let paths = vec![
            PathBuf::from("/a/b/c"),
            PathBuf::from("/a/b"),
            PathBuf::from("/a/b/d"),
        ];
        let result = coalesce_event_paths(paths);
        assert_eq!(result, vec![PathBuf::from("/a/b")]);
    }

    #[test]
    fn coalesce_keeps_siblings() {
        // Sibling paths should both be kept
        let paths = vec![
            PathBuf::from("/a/b"),
            PathBuf::from("/a/c"),
            PathBuf::from("/x/y"),
        ];
        let result = coalesce_event_paths(paths);
        assert_eq!(result.len(), 3);
        assert!(result.contains(&PathBuf::from("/a/b")));
        assert!(result.contains(&PathBuf::from("/a/c")));
        assert!(result.contains(&PathBuf::from("/x/y")));
    }

    #[test]
    fn coalesce_deduplicates_identical_paths() {
        let paths = vec![
            PathBuf::from("/a/b"),
            PathBuf::from("/a/b"),
            PathBuf::from("/a/b"),
        ];
        let result = coalesce_event_paths(paths);
        assert_eq!(result, vec![PathBuf::from("/a/b")]);
    }

    #[test]
    fn coalesce_child_before_ancestor_still_works() {
        // If a child is seen before its ancestor in the input,
        // the algorithm should still only keep the ancestor
        let paths = vec![
            PathBuf::from("/a/b/c/d"),
            PathBuf::from("/a/b/c/e"),
            PathBuf::from("/a/b"), // ancestor seen last
        ];
        let result = coalesce_event_paths(paths);
        assert_eq!(result, vec![PathBuf::from("/a/b")]);
    }

    #[test]
    fn coalesce_similar_prefixes_not_confused() {
        // /foo/bar should NOT be considered an ancestor of /foo/barista
        // because Path::starts_with compares components, not string prefixes
        let paths = vec![PathBuf::from("/foo/bar"), PathBuf::from("/foo/barista")];
        let result = coalesce_event_paths(paths);
        assert_eq!(result.len(), 2);
        assert!(result.contains(&PathBuf::from("/foo/bar")));
        assert!(result.contains(&PathBuf::from("/foo/barista")));
    }

    #[test]
    fn coalesce_deep_nesting() {
        let paths = vec![
            PathBuf::from("/a"),
            PathBuf::from("/a/b"),
            PathBuf::from("/a/b/c"),
            PathBuf::from("/a/b/c/d"),
            PathBuf::from("/a/b/c/d/e"),
        ];
        let result = coalesce_event_paths(paths);
        // Only the shallowest ancestor should remain
        assert_eq!(result, vec![PathBuf::from("/a")]);
    }

    #[test]
    fn coalesce_multiple_independent_subtrees() {
        let paths = vec![
            PathBuf::from("/home/user/project1/src/main.rs"),
            PathBuf::from("/home/user/project1"),
            PathBuf::from("/home/user/project2/docs/readme.md"),
            PathBuf::from("/home/user/project2"),
            PathBuf::from("/tmp/cache"),
        ];
        let result = coalesce_event_paths(paths);
        assert_eq!(result.len(), 3);
        assert!(result.contains(&PathBuf::from("/home/user/project1")));
        assert!(result.contains(&PathBuf::from("/home/user/project2")));
        assert!(result.contains(&PathBuf::from("/tmp/cache")));
    }

    #[test]
    fn has_selected_ancestor_empty_set() {
        let selected = HashSet::new();
        assert!(!has_selected_ancestor(Path::new("/a/b/c"), &selected));
    }

    #[test]
    fn has_selected_ancestor_exact_match() {
        let mut selected = HashSet::new();
        selected.insert(PathBuf::from("/a/b/c"));
        assert!(has_selected_ancestor(Path::new("/a/b/c"), &selected));
    }

    #[test]
    fn has_selected_ancestor_parent_match() {
        let mut selected = HashSet::new();
        selected.insert(PathBuf::from("/a/b"));
        assert!(has_selected_ancestor(Path::new("/a/b/c"), &selected));
        assert!(has_selected_ancestor(Path::new("/a/b/c/d/e"), &selected));
    }

    #[test]
    fn has_selected_ancestor_no_match() {
        let mut selected = HashSet::new();
        selected.insert(PathBuf::from("/x/y"));
        assert!(!has_selected_ancestor(Path::new("/a/b/c"), &selected));
    }

    #[test]
    fn path_depth_basic() {
        assert_eq!(path_depth(Path::new("/")), 1);
        assert_eq!(path_depth(Path::new("/a")), 2);
        assert_eq!(path_depth(Path::new("/a/b")), 3);
        assert_eq!(path_depth(Path::new("/a/b/c")), 4);
    }
}
