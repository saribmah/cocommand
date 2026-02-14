//! Text and wildcard matching utilities.

// ---------------------------------------------------------------------------
// Text segment matching types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub enum TextSegmentMatchKind {
    Substr,
    Prefix,
    Suffix,
    Exact,
}

#[derive(Debug, Clone)]
pub enum TextQuerySegment {
    Concrete(TextSegmentMatcher),
    Star,
    GlobStar,
}

#[derive(Debug, Clone)]
pub struct TextSegmentMatcher {
    kind: TextSegmentMatchKind,
    value: String,
    has_wildcards: bool,
}

impl TextSegmentMatcher {
    pub fn new(kind: TextSegmentMatchKind, value: &str) -> Self {
        Self {
            kind,
            value: value.to_string(),
            has_wildcards: value.contains('*') || value.contains('?'),
        }
    }

    pub fn matches(&self, candidate: &str) -> bool {
        if self.has_wildcards {
            return wildcard_matches(self.value.as_str(), candidate);
        }

        match self.kind {
            TextSegmentMatchKind::Substr => candidate.contains(self.value.as_str()),
            TextSegmentMatchKind::Prefix => candidate.starts_with(self.value.as_str()),
            TextSegmentMatchKind::Suffix => candidate.ends_with(self.value.as_str()),
            TextSegmentMatchKind::Exact => candidate == self.value,
        }
    }
}

// ---------------------------------------------------------------------------
// Path query segmentation
// ---------------------------------------------------------------------------

/// Parses a query text into path segments for matching.
pub fn segment_query_text(raw: &str) -> Vec<TextQuerySegment> {
    let normalized = raw.replace('\\', "/");
    let left_close = normalized.starts_with('/');
    let right_close = normalized.ends_with('/');
    let trimmed = normalized.trim_start_matches('/').trim_end_matches('/');

    if trimmed.is_empty() {
        return Vec::new();
    }

    let segments = trimmed.split('/').collect::<Vec<_>>();
    if segments.iter().any(|segment| segment.is_empty()) {
        return Vec::new();
    }

    let mut kinds = vec![TextSegmentMatchKind::Exact; segments.len()];
    if segments.len() == 1 {
        kinds[0] = if !left_close && !right_close {
            TextSegmentMatchKind::Substr
        } else if !left_close {
            TextSegmentMatchKind::Suffix
        } else if !right_close {
            TextSegmentMatchKind::Prefix
        } else {
            TextSegmentMatchKind::Exact
        };
    } else {
        if !left_close {
            kinds[0] = TextSegmentMatchKind::Suffix;
        }
        if !right_close {
            kinds[segments.len() - 1] = TextSegmentMatchKind::Prefix;
        }
    }

    kinds
        .into_iter()
        .zip(segments)
        .map(|(kind, segment)| {
            if segment == "**" {
                TextQuerySegment::GlobStar
            } else if segment == "*" {
                TextQuerySegment::Star
            } else {
                TextQuerySegment::Concrete(TextSegmentMatcher::new(kind, segment))
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Path query matching
// ---------------------------------------------------------------------------

/// Matches a pattern against path segments.
pub fn path_query_matches(pattern: &[TextQuerySegment], candidate_segments: &[String]) -> bool {
    if pattern.is_empty() || candidate_segments.is_empty() {
        return false;
    }

    let mut current: Option<Vec<usize>> = None;
    let mut pending_globstar = false;
    let mut saw_matcher = false;

    for segment in pattern {
        match segment {
            TextQuerySegment::GlobStar => {
                pending_globstar = true;
            }
            TextQuerySegment::Star => {
                saw_matcher = true;
                let next = if let Some(previous) = current.as_ref() {
                    if pending_globstar {
                        all_descendant_indices(previous.as_slice(), candidate_segments.len())
                    } else {
                        direct_child_indices(previous.as_slice(), candidate_segments.len())
                    }
                } else {
                    all_segment_indices(candidate_segments.len())
                };
                current = Some(next);
                pending_globstar = false;
            }
            TextQuerySegment::Concrete(matcher) => {
                saw_matcher = true;
                let next = if let Some(previous) = current.as_ref() {
                    if pending_globstar {
                        match_descendant_indices(previous.as_slice(), candidate_segments, matcher)
                    } else {
                        match_direct_child_indices(previous.as_slice(), candidate_segments, matcher)
                    }
                } else {
                    match_initial_indices(candidate_segments, matcher)
                };
                current = Some(next);
                pending_globstar = false;
            }
        }
    }

    let final_matches = if pending_globstar {
        if let Some(previous) = current.as_ref() {
            all_descendant_indices(previous.as_slice(), candidate_segments.len())
        } else {
            all_segment_indices(candidate_segments.len())
        }
    } else if saw_matcher {
        current.unwrap_or_default()
    } else {
        all_segment_indices(candidate_segments.len())
    };

    !final_matches.is_empty()
}

// ---------------------------------------------------------------------------
// Wildcard matching
// ---------------------------------------------------------------------------

/// Matches a pattern with wildcards (* and ?) against a candidate string.
pub fn wildcard_matches(pattern: &str, candidate: &str) -> bool {
    let pattern_chars = pattern.chars().collect::<Vec<_>>();
    let candidate_chars = candidate.chars().collect::<Vec<_>>();

    let mut pattern_index = 0usize;
    let mut candidate_index = 0usize;
    let mut star_index: Option<usize> = None;
    let mut star_candidate_index = 0usize;

    while candidate_index < candidate_chars.len() {
        if pattern_index < pattern_chars.len()
            && (pattern_chars[pattern_index] == '?'
                || pattern_chars[pattern_index] == candidate_chars[candidate_index])
        {
            pattern_index += 1;
            candidate_index += 1;
            continue;
        }

        if pattern_index < pattern_chars.len() && pattern_chars[pattern_index] == '*' {
            star_index = Some(pattern_index);
            pattern_index += 1;
            star_candidate_index = candidate_index;
            continue;
        }

        if let Some(last_star_index) = star_index {
            pattern_index = last_star_index + 1;
            star_candidate_index += 1;
            candidate_index = star_candidate_index;
            continue;
        }

        return false;
    }

    while pattern_index < pattern_chars.len() && pattern_chars[pattern_index] == '*' {
        pattern_index += 1;
    }

    pattern_index == pattern_chars.len()
}

// ---------------------------------------------------------------------------
// Index helpers
// ---------------------------------------------------------------------------

fn all_segment_indices(path_len: usize) -> Vec<usize> {
    (0..path_len).collect()
}

fn direct_child_indices(parents: &[usize], path_len: usize) -> Vec<usize> {
    dedup_indices(
        path_len,
        parents.iter().filter_map(|index| index.checked_add(1)),
    )
}

fn all_descendant_indices(parents: &[usize], path_len: usize) -> Vec<usize> {
    dedup_indices(
        path_len,
        parents.iter().flat_map(|index| (index + 1)..path_len),
    )
}

fn match_initial_indices(path: &[String], matcher: &TextSegmentMatcher) -> Vec<usize> {
    path.iter()
        .enumerate()
        .filter_map(|(index, segment)| matcher.matches(segment.as_str()).then_some(index))
        .collect()
}

fn match_direct_child_indices(
    parents: &[usize],
    path: &[String],
    matcher: &TextSegmentMatcher,
) -> Vec<usize> {
    dedup_indices(
        path.len(),
        parents.iter().filter_map(|index| index.checked_add(1)),
    )
    .into_iter()
    .filter(|index| matcher.matches(path[*index].as_str()))
    .collect()
}

fn match_descendant_indices(
    parents: &[usize],
    path: &[String],
    matcher: &TextSegmentMatcher,
) -> Vec<usize> {
    dedup_indices(
        path.len(),
        parents.iter().flat_map(|index| (index + 1)..path.len()),
    )
    .into_iter()
    .filter(|index| matcher.matches(path[*index].as_str()))
    .collect()
}

fn dedup_indices<I>(path_len: usize, indices: I) -> Vec<usize>
where
    I: IntoIterator<Item = usize>,
{
    if path_len == 0 {
        return Vec::new();
    }

    let mut seen = vec![false; path_len];
    let mut deduped = Vec::new();
    for index in indices {
        if index >= path_len || seen[index] {
            continue;
        }
        seen[index] = true;
        deduped.push(index);
    }
    deduped
}

// ---------------------------------------------------------------------------
// Text entry matching
// ---------------------------------------------------------------------------

/// Matches a text value against an entry's name and path.
pub fn text_matches(value: &str, name: &str, path: &str, path_segments: &[String]) -> bool {
    if value.is_empty() {
        return true;
    }

    let query_segments = segment_query_text(value);
    if !query_segments.is_empty() {
        return path_query_matches(query_segments.as_slice(), path_segments);
    }

    if value.contains('/') || value.contains('\\') {
        return false;
    }

    name.contains(value) || path.contains(value)
}

/// Checks if a term is suitable for name-based prefiltering.
pub fn is_name_prefilter_term(raw: &str) -> bool {
    let trimmed = raw.trim();
    !trimmed.is_empty()
        && !trimmed.contains('/')
        && !trimmed.contains('\\')
        && !trimmed.contains('*')
        && !trimmed.contains('?')
}
