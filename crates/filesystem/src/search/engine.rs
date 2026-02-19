//! Search and query evaluation for the filesystem index.

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::cancel::CancellationToken;
use crate::error::Result;
use crate::indexer::{NodeView, RootIndexData};
use crate::query::{
    file_content_matches, segment_query_text, QueryExpression, QueryFilter, QueryTerm,
    SearchQueryMatcher, TextQuerySegment, TextSegmentMatcher, TypeFilterTarget,
};
use crate::storage::{NodeFileType, SlabIndex};
use crate::types::{FileEntry, FileType, KindFilter, SearchResult};

/// Threshold for switching from iterating file metadata to using Spotlight (mdfind).
/// When the candidate set exceeds this size, Spotlight's indexed search is faster than
/// reading xattr metadata for each file individually.
const TAG_FILTER_MDFIND_THRESHOLD: usize = 10000;

/// A candidate entry for search matching.
/// Keeps only the node id to avoid eager path reconstruction.
struct SearchCandidate {
    id: SlabIndex,
}

/// Searches the index data with the given query.
///
/// The `cancel_token` enables early termination when a newer search starts.
/// Returns `None` if the search was cancelled, otherwise returns the results.
#[allow(clippy::too_many_arguments)]
pub fn search_index_data(
    root: &Path,
    data: &RootIndexData,
    query: String,
    kind: KindFilter,
    include_hidden: bool,
    case_sensitive: bool,
    max_results: usize,
    max_depth: usize,
    index_state: &str,
    index_scanned_files: usize,
    index_scanned_dirs: usize,
    index_started_at: Option<u64>,
    index_last_update_at: Option<u64>,
    index_finished_at: Option<u64>,
    cancel_token: CancellationToken,
) -> Result<Option<SearchResult>> {
    let matcher = SearchQueryMatcher::compile(&query, case_sensitive)?;
    let required_terms = matcher.required_name_terms();
    let candidate_ids =
        candidate_node_ids_for_terms(data, &required_terms, case_sensitive, cancel_token);

    // Check for cancellation after term matching
    if cancel_token.is_cancelled().is_none() {
        return Ok(None);
    }

    let prefiltered_ids = candidate_ids.map(|ids| ids.into_iter().collect::<BTreeSet<_>>());

    let mut candidates: Vec<SearchCandidate> = Vec::new();
    let Some(root_id) = data.node_index_for_path(root) else {
        return Ok(Some(SearchResult {
            query,
            root: root.to_string_lossy().to_string(),
            entries: Vec::new(),
            count: 0,
            truncated: false,
            scanned: 0,
            errors: data.errors,
            index_state: index_state.to_string(),
            index_scanned_files,
            index_scanned_dirs,
            index_started_at,
            index_last_update_at,
            index_finished_at,
            highlight_terms: matcher.highlight_terms(),
        }));
    };

    // Traverse only the indexed root subtree. This avoids repeatedly computing
    // full parent chains for every node on each search.
    let mut stack = vec![(root_id, 0usize, false)];
    let mut counter = 0usize;
    while let Some((id, depth, hidden_in_chain)) = stack.pop() {
        if cancel_token.is_cancelled_sparse(counter).is_none() {
            return Ok(None);
        }
        counter += 1;

        if depth > max_depth {
            continue;
        }

        let Some(node) = data.get_node(id) else {
            continue;
        };

        // Indexed root visibility is always allowed, even if hidden. For all
        // descendants, hide matches where any node in the path is hidden.
        let hidden_for_this = hidden_in_chain || (depth > 0 && node.is_hidden());
        if !include_hidden && hidden_for_this {
            continue;
        }

        if let Some(prefilter) = prefiltered_ids.as_ref() {
            if prefilter.contains(&id) {
                candidates.push(SearchCandidate { id });
            }
        } else {
            candidates.push(SearchCandidate { id });
        }

        if depth == max_depth {
            continue;
        }
        for child_id in node.children.iter().rev() {
            stack.push((*child_id, depth + 1, hidden_for_this));
        }
    }

    // Check for cancellation before expression evaluation
    if cancel_token.is_cancelled().is_none() {
        return Ok(None);
    }

    let universe = candidates.iter().map(|c| c.id).collect::<BTreeSet<_>>();
    let matched_ids = match evaluate_expression_set(
        data,
        &matcher,
        matcher.expression(),
        &candidates,
        &universe,
        cancel_token,
    ) {
        Some(ids) => ids,
        None => return Ok(None), // Cancelled
    };
    let scanned = candidates.len();

    // Check for cancellation before building results
    if cancel_token.is_cancelled().is_none() {
        return Ok(None);
    }

    let mut matched_nodes = Vec::new();
    for (i, candidate) in candidates.into_iter().enumerate() {
        // Sparse check during result building
        if cancel_token.is_cancelled_sparse(i).is_none() {
            return Ok(None);
        }

        if !matched_ids.contains(&candidate.id) {
            continue;
        }
        let Some(node) = data.get_node(candidate.id) else {
            continue;
        };
        let file_type = match node.file_type() {
            NodeFileType::File => FileType::File,
            NodeFileType::Dir => FileType::Directory,
            NodeFileType::Symlink => FileType::Symlink,
            NodeFileType::Unknown => FileType::Other,
        };
        if !kind.matches(file_type) {
            continue;
        }
        matched_nodes.push((
            node.name().to_string(),
            file_type,
            candidate.id,
            node.size(),
            node.modified_at(),
        ));
    }

    // Keep sorting behavior aligned with Cardinal while avoiding eager path expansion.
    matched_nodes.sort_by(|a, b| a.0.cmp(&b.0));
    let truncated = matched_nodes.len() > max_results;
    if truncated {
        matched_nodes.truncate(max_results);
    }

    let mut matches = Vec::with_capacity(matched_nodes.len());
    for (i, (name, file_type, id, mut size, mut modified_at)) in
        matched_nodes.into_iter().enumerate()
    {
        if cancel_token.is_cancelled_sparse(i).is_none() {
            return Ok(None);
        }
        let Some(path) = compute_node_path(data, id) else {
            continue;
        };

        // Cardinal doesn't force file metadata during full indexing. If metadata is
        // absent for this node, fetch it lazily for the final result entry only.
        if modified_at.is_none() && matches!(file_type, FileType::File | FileType::Symlink) {
            if let Ok(fs_meta) = std::fs::symlink_metadata(&path) {
                size = Some(fs_meta.len());
                modified_at = fs_meta
                    .modified()
                    .ok()
                    .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                    .map(|value| value.as_secs());
            }
        }

        matches.push(FileEntry {
            path,
            name,
            file_type,
            size,
            modified_at,
        });
    }

    Ok(Some(SearchResult {
        query,
        root: root.to_string_lossy().to_string(),
        count: matches.len(),
        entries: matches,
        truncated,
        scanned,
        errors: data.errors,
        index_state: index_state.to_string(),
        index_scanned_files,
        index_scanned_dirs,
        index_started_at,
        index_last_update_at,
        index_finished_at,
        highlight_terms: matcher.highlight_terms(),
    }))
}

/// Finds candidate node IDs based on required search terms.
fn candidate_node_ids_for_terms(
    data: &RootIndexData,
    required_terms: &[String],
    case_sensitive: bool,
    cancel_token: CancellationToken,
) -> Option<Vec<SlabIndex>> {
    if required_terms.is_empty() {
        return None;
    }

    let mut intersection: Option<Vec<SlabIndex>> = None;
    for term in required_terms {
        let mut matched = Vec::new();
        for (i, (name, ids)) in data.name_index.iter().enumerate() {
            // Sparse cancellation check during name index iteration
            if cancel_token.is_cancelled_sparse(i).is_none() {
                return Some(Vec::new()); // Return empty on cancellation
            }
            let matches = if case_sensitive {
                name.contains(term)
            } else {
                contains_ascii_case_insensitive(name, term)
            };
            if matches {
                matched.extend(ids.iter().copied());
            }
        }
        matched.sort_unstable();
        matched.dedup();

        if matched.is_empty() {
            return Some(Vec::new());
        }

        intersection = Some(match intersection {
            None => matched,
            Some(existing) => intersect_sorted_ids(existing, matched, cancel_token),
        });
    }

    intersection
}

fn contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    let haystack_bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();
    if needle_bytes.is_empty() {
        return true;
    }
    if needle_bytes.len() > haystack_bytes.len() {
        return false;
    }
    haystack_bytes
        .windows(needle_bytes.len())
        .any(|window| window.eq_ignore_ascii_case(needle_bytes))
}

/// Intersects two sorted ID vectors.
fn intersect_sorted_ids(
    mut left: Vec<SlabIndex>,
    mut right: Vec<SlabIndex>,
    cancel_token: CancellationToken,
) -> Vec<SlabIndex> {
    left.sort_unstable();
    left.dedup();
    right.sort_unstable();
    right.dedup();

    let mut result = Vec::new();
    let mut i = 0usize;
    let mut j = 0usize;
    let mut counter = 0usize;
    while i < left.len() && j < right.len() {
        // Sparse cancellation check
        if cancel_token.is_cancelled_sparse(counter).is_none() {
            return result; // Return partial result on cancellation
        }
        counter += 1;

        if left[i] == right[j] {
            result.push(left[i]);
            i += 1;
            j += 1;
        } else if left[i] < right[j] {
            i += 1;
        } else {
            j += 1;
        }
    }
    result
}

fn compute_node_path(data: &RootIndexData, id: SlabIndex) -> Option<String> {
    let view = NodeView::new(&*data.file_nodes, id);
    view.compute_path("")
}

fn all_indexed_ids(
    data: &RootIndexData,
    cancel_token: CancellationToken,
) -> Option<Vec<SlabIndex>> {
    let mut result = Vec::new();
    for (i, indices) in data.name_index.values().enumerate() {
        cancel_token.is_cancelled_sparse(i)?;
        result.extend(indices.iter().copied());
    }
    Some(result)
}

fn matcher_matches_name(matcher: &TextSegmentMatcher, name: &str, case_sensitive: bool) -> bool {
    if case_sensitive {
        matcher.matches(name)
    } else {
        matcher.matches(&name.to_ascii_lowercase())
    }
}

fn evaluate_text_term_set(
    data: &RootIndexData,
    value: &str,
    case_sensitive: bool,
    universe: &BTreeSet<SlabIndex>,
    cancel_token: CancellationToken,
) -> Option<BTreeSet<SlabIndex>> {
    let segments = segment_query_text(value);
    if segments.is_empty() {
        return Some(BTreeSet::new());
    }

    let matched = execute_text_segments(data, &segments, case_sensitive, cancel_token)?;
    Some(
        matched
            .into_iter()
            .filter(|id| universe.contains(id))
            .collect(),
    )
}

fn execute_text_segments(
    data: &RootIndexData,
    segments: &[TextQuerySegment],
    case_sensitive: bool,
    cancel_token: CancellationToken,
) -> Option<Vec<SlabIndex>> {
    let mut node_set: Option<Vec<SlabIndex>> = None;
    let mut pending_globstar = false;
    let mut saw_matcher = false;
    let mut saw_globstar = false;

    for segment in segments {
        match segment {
            TextQuerySegment::GlobStar => {
                saw_globstar = true;
                pending_globstar = true;
            }
            TextQuerySegment::Star => {
                saw_matcher = true;
                let new_node_set = if let Some(nodes) = &node_set {
                    if pending_globstar {
                        all_descendant_segments(data, nodes, cancel_token)
                    } else {
                        all_direct_children(data, nodes, cancel_token)
                    }
                } else {
                    all_indexed_ids(data, cancel_token)
                }?;
                node_set = Some(new_node_set);
                pending_globstar = false;
            }
            TextQuerySegment::Concrete(matcher) => {
                saw_matcher = true;
                let new_node_set = if let Some(nodes) = &node_set {
                    if pending_globstar {
                        match_descendant_segments(
                            data,
                            nodes,
                            matcher,
                            case_sensitive,
                            cancel_token,
                        )
                    } else {
                        match_direct_child_segments(
                            data,
                            nodes,
                            matcher,
                            case_sensitive,
                            cancel_token,
                        )
                    }
                } else {
                    match_initial_segment(data, matcher, case_sensitive, cancel_token)
                }?;
                node_set = Some(new_node_set);
                pending_globstar = false;
            }
        }
    }

    let mut nodes = if pending_globstar {
        if let Some(nodes) = node_set.take() {
            Some(all_descendant_segments(data, &nodes, cancel_token)?)
        } else {
            all_indexed_ids(data, cancel_token)
        }
    } else if saw_matcher {
        node_set
    } else {
        all_indexed_ids(data, cancel_token)
    };

    // `**` can produce duplicate hits for the same descendant; keep one.
    if saw_globstar && saw_matcher {
        if let Some(nodes) = &mut nodes {
            dedup_indices_in_place(nodes);
        }
    }

    nodes
}

fn match_initial_segment(
    data: &RootIndexData,
    matcher: &TextSegmentMatcher,
    case_sensitive: bool,
    cancel_token: CancellationToken,
) -> Option<Vec<SlabIndex>> {
    let mut nodes = Vec::new();
    for (i, (name, ids)) in data.name_index.iter().enumerate() {
        cancel_token.is_cancelled_sparse(i)?;
        if matcher_matches_name(matcher, name, case_sensitive) {
            nodes.extend(ids.iter().copied());
        }
    }
    Some(nodes)
}

fn match_direct_child_segments(
    data: &RootIndexData,
    parents: &[SlabIndex],
    matcher: &TextSegmentMatcher,
    case_sensitive: bool,
    cancel_token: CancellationToken,
) -> Option<Vec<SlabIndex>> {
    let mut new_node_set = Vec::new();
    for (i, &node) in parents.iter().enumerate() {
        cancel_token.is_cancelled_sparse(i)?;
        let Some(parent_node) = data.get_node(node) else {
            continue;
        };
        let mut child_matches: Vec<(&'static str, SlabIndex)> = parent_node
            .children
            .iter()
            .filter_map(|&child| {
                let child_node = data.get_node(child)?;
                let name = child_node.name();
                if matcher_matches_name(matcher, name, case_sensitive) {
                    Some((name, child))
                } else {
                    None
                }
            })
            .collect();
        child_matches.sort_unstable_by_key(|(name, _)| *name);
        new_node_set.extend(child_matches.into_iter().map(|(_, index)| index));
    }
    Some(new_node_set)
}

fn all_direct_children(
    data: &RootIndexData,
    parents: &[SlabIndex],
    cancel_token: CancellationToken,
) -> Option<Vec<SlabIndex>> {
    let mut new_node_set = Vec::new();
    for (i, &node) in parents.iter().enumerate() {
        cancel_token.is_cancelled_sparse(i)?;
        let Some(parent_node) = data.get_node(node) else {
            continue;
        };
        let mut child_matches: Vec<(&'static str, SlabIndex)> = parent_node
            .children
            .iter()
            .filter_map(|&child| {
                data.get_node(child)
                    .map(|child_node| (child_node.name(), child))
            })
            .collect();
        child_matches.sort_unstable_by_key(|(name, _)| *name);
        new_node_set.extend(child_matches.into_iter().map(|(_, index)| index));
    }
    Some(new_node_set)
}

fn match_descendant_segments(
    data: &RootIndexData,
    parents: &[SlabIndex],
    matcher: &TextSegmentMatcher,
    case_sensitive: bool,
    cancel_token: CancellationToken,
) -> Option<Vec<SlabIndex>> {
    let mut matches = Vec::new();
    let mut visited = 0usize;
    for &node in parents {
        cancel_token.is_cancelled_sparse(visited)?;
        let descendants = all_subnodes(data, node, cancel_token)?;
        for descendant in descendants {
            cancel_token.is_cancelled_sparse(visited)?;
            visited += 1;
            let Some(descendant_node) = data.get_node(descendant) else {
                continue;
            };
            let name = descendant_node.name();
            if matcher_matches_name(matcher, name, case_sensitive) {
                matches.push((name, descendant));
            }
        }
    }
    matches.sort_unstable_by_key(|(name, _)| *name);
    Some(matches.into_iter().map(|(_, index)| index).collect())
}

fn all_descendant_segments(
    data: &RootIndexData,
    parents: &[SlabIndex],
    cancel_token: CancellationToken,
) -> Option<Vec<SlabIndex>> {
    let mut matches = Vec::new();
    let mut visited = 0usize;
    for &node in parents {
        cancel_token.is_cancelled_sparse(visited)?;
        let descendants = all_subnodes(data, node, cancel_token)?;
        for descendant in descendants {
            cancel_token.is_cancelled_sparse(visited)?;
            visited += 1;
            let Some(descendant_node) = data.get_node(descendant) else {
                continue;
            };
            matches.push((descendant_node.name(), descendant));
        }
    }
    matches.sort_unstable_by_key(|(name, _)| *name);
    Some(matches.into_iter().map(|(_, index)| index).collect())
}

fn all_subnodes(
    data: &RootIndexData,
    index: SlabIndex,
    cancel_token: CancellationToken,
) -> Option<Vec<SlabIndex>> {
    let mut result = Vec::new();
    let mut stack = Vec::new();
    let mut i = 0usize;

    if let Some(node) = data.get_node(index) {
        stack.extend(node.children.iter().copied());
    }

    while let Some(current) = stack.pop() {
        cancel_token.is_cancelled_sparse(i)?;
        i += 1;
        result.push(current);
        if let Some(node) = data.get_node(current) {
            stack.extend(node.children.iter().copied());
        }
    }
    Some(result)
}

fn dedup_indices_in_place(indices: &mut Vec<SlabIndex>) {
    indices.sort_unstable();
    indices.dedup();
}

/// Collects extension IDs from the index.
fn collect_extension_ids(data: &RootIndexData, extensions: &[String]) -> BTreeSet<SlabIndex> {
    let mut ids = BTreeSet::new();
    for extension in extensions {
        ids.extend(data.indices_for_extension(extension.as_str()));
    }
    ids
}

/// Gets IDs for a type filter target.
fn type_target_ids(data: &RootIndexData, target: &TypeFilterTarget) -> BTreeSet<SlabIndex> {
    match target {
        TypeFilterTarget::File => data.file_ids(),
        TypeFilterTarget::Directory => data.directory_ids(),
        TypeFilterTarget::Extensions(extensions) => {
            let values = extensions
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>();
            collect_extension_ids(data, &values)
        }
    }
}

/// Returns a prefilter set for a filter if applicable.
fn prefilter_set_for_filter(
    data: &RootIndexData,
    filter: &QueryFilter,
) -> Option<BTreeSet<SlabIndex>> {
    match filter {
        QueryFilter::Extension(extensions) => Some(collect_extension_ids(data, extensions)),
        QueryFilter::Type(target) => Some(type_target_ids(data, target)),
        QueryFilter::TypeMacro { target, .. } => Some(type_target_ids(data, target)),
        QueryFilter::File { .. } => Some(data.file_ids()),
        QueryFilter::Folder { .. } => Some(data.directory_ids()),
        _ => None,
    }
}

/// Checks if a filter is an exact prefilter.
fn is_exact_prefilter_filter(filter: &QueryFilter) -> bool {
    match filter {
        QueryFilter::Extension(_) => true,
        QueryFilter::Type(_) => true,
        QueryFilter::TypeMacro { argument, .. } => argument.is_none(),
        QueryFilter::File { argument } => argument.is_none(),
        QueryFilter::Folder { argument } => argument.is_none(),
        _ => false,
    }
}

/// Returns a structural filter set (parent:, infolder:, nosubfolders:).
/// Returns `None` if the filter doesn't apply (not a structural filter).
fn structural_filter_set(
    data: &RootIndexData,
    matcher: &SearchQueryMatcher,
    filter: &QueryFilter,
    universe: &BTreeSet<SlabIndex>,
    cancel_token: CancellationToken,
) -> Option<BTreeSet<SlabIndex>> {
    match filter {
        QueryFilter::Parent { path } => {
            let parent_id = data.node_id_for_path(path.as_str(), matcher.case_sensitive())?;
            let node = data.get_node(parent_id)?;
            Some(
                node.children
                    .iter()
                    .copied()
                    .filter(|id| universe.contains(id))
                    .collect(),
            )
        }
        QueryFilter::InFolder { path } => {
            let folder_id = data.node_id_for_path(path.as_str(), matcher.case_sensitive())?;
            let mut stack = vec![folder_id];
            let mut descendants = BTreeSet::new();
            let mut counter = 0usize;

            while let Some(current_id) = stack.pop() {
                // Sparse cancellation check during tree traversal
                if cancel_token.is_cancelled_sparse(counter).is_none() {
                    return Some(descendants); // Return partial on cancel
                }
                counter += 1;

                let Some(node) = data.get_node(current_id) else {
                    continue;
                };
                for child_id in &node.children {
                    if universe.contains(child_id) {
                        descendants.insert(*child_id);
                    }
                    stack.push(*child_id);
                }
            }

            Some(descendants)
        }
        QueryFilter::NoSubfolders { path } => {
            let folder_id = data.node_id_for_path(path.as_str(), matcher.case_sensitive())?;
            let Some(node) = data.get_node(folder_id) else {
                return Some(BTreeSet::new());
            };
            let mut result = BTreeSet::new();
            if universe.contains(&folder_id) {
                result.insert(folder_id);
            }
            for child_id in &node.children {
                // Check if child is a file by looking at the node's file type
                if let Some(child_node) = data.get_node(*child_id) {
                    if child_node.is_file() && universe.contains(child_id) {
                        result.insert(*child_id);
                    }
                }
            }
            Some(result)
        }
        _ => None,
    }
}

/// Evaluates a content filter using parallel file content search.
/// Returns `None` if cancelled.
fn evaluate_content_filter(
    data: &RootIndexData,
    needle: &str,
    candidates: &[SearchCandidate],
    universe: &BTreeSet<SlabIndex>,
    case_insensitive: bool,
    cancel_token: CancellationToken,
) -> Option<BTreeSet<SlabIndex>> {
    cancel_token.is_cancelled()?;

    // Prepare needle bytes (already lowercased if case_insensitive by parser)
    let needle_bytes = needle.as_bytes();
    if needle_bytes.is_empty() {
        return Some(BTreeSet::new());
    }

    // Filter to only files in the universe
    let file_candidates: Vec<(SlabIndex, PathBuf)> = candidates
        .iter()
        .filter(|c| universe.contains(&c.id))
        .filter_map(|c| {
            let node = data.get_node(c.id)?;
            if node.file_type() != NodeFileType::File {
                return None;
            }
            let full_path = data.node_path(c.id)?;
            Some((c.id, full_path))
        })
        .collect();

    // Use rayon's par_bridge for parallel file content search
    let matched_indices: Vec<SlabIndex> = file_candidates
        .into_iter()
        .par_bridge()
        .filter_map(|(id, path)| {
            let matches =
                file_content_matches(&path, needle_bytes, case_insensitive, cancel_token)?;
            matches.then_some(id)
        })
        .collect();

    // Check cancellation after parallel work
    cancel_token.is_cancelled()?;

    Some(matched_indices.into_iter().collect())
}

/// Evaluates a tag filter using adaptive strategy based on candidate set size.
/// Returns `None` if cancelled.
///
/// This function uses two strategies based on the size of the candidate set:
/// - **Small sets (≤ TAG_FILTER_MDFIND_THRESHOLD)**: Read xattr metadata for each file in parallel.
///   This is efficient when there are few candidates since we avoid spawning mdfind.
/// - **Large sets (> TAG_FILTER_MDFIND_THRESHOLD)**: Use Spotlight's `mdfind` command to quickly
///   find all files with matching tags, then intersect with the candidate set. This leverages
///   Spotlight's pre-indexed tag data for better performance on large datasets.
fn evaluate_tag_filter(
    data: &RootIndexData,
    tags: &[String],
    candidates: &[SearchCandidate],
    universe: &BTreeSet<SlabIndex>,
    case_insensitive: bool,
    cancel_token: CancellationToken,
) -> Option<BTreeSet<SlabIndex>> {
    cancel_token.is_cancelled()?;

    if tags.is_empty() {
        return Some(BTreeSet::new());
    }

    // Filter candidates to those in the universe and build full paths
    let file_candidates: Vec<(SlabIndex, PathBuf)> = candidates
        .iter()
        .filter(|c| universe.contains(&c.id))
        .filter_map(|c| {
            // Get node to verify it exists
            let _node = data.get_node(c.id)?;
            let full_path = data.node_path(c.id)?;
            Some((c.id, full_path))
        })
        .collect();

    cancel_token.is_cancelled()?;

    // Adaptive strategy: use xattr for small sets, mdfind for large sets
    let matched_indices: BTreeSet<SlabIndex> =
        if file_candidates.len() <= TAG_FILTER_MDFIND_THRESHOLD {
            // Small set: read xattr metadata for each file in parallel
            evaluate_tag_filter_via_xattr(&file_candidates, tags, case_insensitive, cancel_token)?
        } else {
            // Large set: use mdfind to quickly narrow down, then intersect
            evaluate_tag_filter_via_mdfind(
                data,
                &file_candidates,
                tags,
                case_insensitive,
                cancel_token,
            )?
        };

    // Check cancellation after evaluation
    cancel_token.is_cancelled()?;

    Some(matched_indices)
}

/// Evaluates tag filter by reading xattr metadata for each file.
/// Used for small candidate sets where spawning mdfind would be overkill.
fn evaluate_tag_filter_via_xattr(
    file_candidates: &[(SlabIndex, PathBuf)],
    tags: &[String],
    case_insensitive: bool,
    cancel_token: CancellationToken,
) -> Option<BTreeSet<SlabIndex>> {
    use crate::watcher::file_has_any_tag;

    let matched_indices: Vec<SlabIndex> = file_candidates
        .iter()
        .par_bridge()
        .filter_map(|(id, path)| {
            // Check for cancellation (coarse-grained since par_bridge handles distribution)
            cancel_token.is_cancelled()?;
            // Check if file has any of the requested tags
            if file_has_any_tag(path, tags, case_insensitive) {
                Some(*id)
            } else {
                None
            }
        })
        .collect();

    // Check cancellation after parallel work
    cancel_token.is_cancelled()?;

    Some(matched_indices.into_iter().collect())
}

/// Evaluates tag filter by using Spotlight's mdfind command.
/// Used for large candidate sets where mdfind's indexed search is faster.
#[cfg(target_os = "macos")]
fn evaluate_tag_filter_via_mdfind(
    _data: &RootIndexData,
    file_candidates: &[(SlabIndex, PathBuf)],
    tags: &[String],
    case_insensitive: bool,
    cancel_token: CancellationToken,
) -> Option<BTreeSet<SlabIndex>> {
    use crate::watcher::search_tags_mdfind;

    cancel_token.is_cancelled()?;

    // Use mdfind to get all files with matching tags across the filesystem
    let spotlight_paths = match search_tags_mdfind(tags.to_vec(), case_insensitive) {
        Ok(paths) => paths,
        Err(e) => {
            // If mdfind fails (e.g., forbidden characters, command not available),
            // fall back to xattr-based filtering
            tracing::debug!("mdfind failed, falling back to xattr: {}", e);
            return evaluate_tag_filter_via_xattr(
                file_candidates,
                tags,
                case_insensitive,
                cancel_token,
            );
        }
    };

    cancel_token.is_cancelled()?;

    // Build a set of paths returned by mdfind for fast lookup
    let spotlight_path_set: HashSet<PathBuf> = spotlight_paths.into_iter().collect();

    // Intersect with our candidate set
    let matched_indices: BTreeSet<SlabIndex> = file_candidates
        .iter()
        .filter_map(|(id, path)| {
            // Check if the path is in the mdfind results
            // We need to handle path normalization since mdfind returns absolute paths
            let canonical = path.canonicalize().ok()?;
            if spotlight_path_set.contains(&canonical) || spotlight_path_set.contains(path) {
                Some(*id)
            } else {
                None
            }
        })
        .collect();

    Some(matched_indices)
}

/// Fallback for non-macOS platforms: always use xattr-based filtering.
#[cfg(not(target_os = "macos"))]
fn evaluate_tag_filter_via_mdfind(
    _data: &RootIndexData,
    file_candidates: &[(SlabIndex, PathBuf)],
    tags: &[String],
    case_insensitive: bool,
    cancel_token: CancellationToken,
) -> Option<BTreeSet<SlabIndex>> {
    // On non-macOS, tags don't exist, but we fall back to xattr check
    // which will return empty results anyway
    evaluate_tag_filter_via_xattr(file_candidates, tags, case_insensitive, cancel_token)
}

/// Evaluates a single term against candidates.
/// Returns `None` if cancelled.
fn matches_term_with_path(
    data: &RootIndexData,
    matcher: &SearchQueryMatcher,
    term: &QueryTerm,
    candidate_id: SlabIndex,
) -> bool {
    let Some(node) = data.get_node(candidate_id) else {
        return false;
    };
    let Some(path) = compute_node_path(data, candidate_id) else {
        return false;
    };
    matcher.matches_node_term(term, node, &path)
}

fn evaluate_term_set(
    data: &RootIndexData,
    matcher: &SearchQueryMatcher,
    term: &QueryTerm,
    candidates: &[SearchCandidate],
    universe: &BTreeSet<SlabIndex>,
    cancel_token: CancellationToken,
) -> Option<BTreeSet<SlabIndex>> {
    if let QueryTerm::Text(value) = term {
        return evaluate_text_term_set(
            data,
            value,
            matcher.case_sensitive(),
            universe,
            cancel_token,
        );
    }

    if let QueryTerm::Filter(filter) = term {
        // Handle content filter specially with parallel search
        if let QueryFilter::Content { needle } = filter {
            return evaluate_content_filter(
                data,
                needle,
                candidates,
                universe,
                !matcher.case_sensitive(),
                cancel_token,
            );
        }

        // Handle tag filter with parallel file tag reading
        if let QueryFilter::Tag { tags } = filter {
            return evaluate_tag_filter(
                data,
                tags,
                candidates,
                universe,
                !matcher.case_sensitive(),
                cancel_token,
            );
        }

        if let Some(set) = structural_filter_set(data, matcher, filter, universe, cancel_token) {
            return Some(set);
        }

        if let Some(prefilter) = prefilter_set_for_filter(data, filter) {
            let narrowed = prefilter
                .intersection(universe)
                .copied()
                .collect::<BTreeSet<_>>();
            if is_exact_prefilter_filter(filter) {
                return Some(narrowed);
            }

            let mut result = BTreeSet::new();
            for (i, candidate) in candidates.iter().enumerate() {
                cancel_token.is_cancelled_sparse(i)?;
                if !narrowed.contains(&candidate.id) {
                    continue;
                }
                if matches_term_with_path(data, matcher, term, candidate.id) {
                    result.insert(candidate.id);
                }
            }
            return Some(result);
        }
    }

    let mut result = BTreeSet::new();
    for (i, candidate) in candidates.iter().enumerate() {
        cancel_token.is_cancelled_sparse(i)?;
        if matches_term_with_path(data, matcher, term, candidate.id) {
            result.insert(candidate.id);
        }
    }
    Some(result)
}

/// Evaluates a query expression against candidates.
/// Returns `None` if cancelled.
fn evaluate_expression_set(
    data: &RootIndexData,
    matcher: &SearchQueryMatcher,
    expression: &QueryExpression,
    candidates: &[SearchCandidate],
    universe: &BTreeSet<SlabIndex>,
    cancel_token: CancellationToken,
) -> Option<BTreeSet<SlabIndex>> {
    // Check cancellation at the start of each expression evaluation
    cancel_token.is_cancelled()?;

    match expression {
        QueryExpression::Term(term) => {
            evaluate_term_set(data, matcher, term, candidates, universe, cancel_token)
        }
        QueryExpression::Not(inner) => {
            let inner_set =
                evaluate_expression_set(data, matcher, inner, candidates, universe, cancel_token)?;
            Some(universe.difference(&inner_set).copied().collect())
        }
        QueryExpression::And(parts) => {
            let mut parts_iter = parts.iter();
            let Some(first) = parts_iter.next() else {
                return Some(universe.clone());
            };
            let mut set =
                evaluate_expression_set(data, matcher, first, candidates, universe, cancel_token)?;
            for part in parts_iter {
                cancel_token.is_cancelled()?;
                let other = evaluate_expression_set(
                    data,
                    matcher,
                    part,
                    candidates,
                    universe,
                    cancel_token,
                )?;
                set = set.intersection(&other).copied().collect();
                if set.is_empty() {
                    break;
                }
            }
            Some(set)
        }
        QueryExpression::Or(parts) => {
            let mut set = BTreeSet::new();
            for part in parts {
                cancel_token.is_cancelled()?;
                let other = evaluate_expression_set(
                    data,
                    matcher,
                    part,
                    candidates,
                    universe,
                    cancel_token,
                )?;
                set = set.union(&other).copied().collect();
            }
            Some(set)
        }
    }
}
