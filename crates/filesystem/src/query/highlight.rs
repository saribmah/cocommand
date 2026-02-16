//! Highlight term extraction for search results.
//!
//! Extracts terms from a query expression that should be highlighted in search results.
//! `derive_highlight_terms` approach:
//! - Collects text terms and filter arguments
//! - Splits on wildcards (* and ?) to get literal chunks
//! - Lowercases all terms for case-insensitive matching
//! - Deduplicates using BTreeSet for sorted, unique output

use std::collections::BTreeSet;

use super::expression::{QueryExpression, QueryFilter, QueryTerm};

/// Derives highlight terms from a query expression.
///
/// Returns a sorted, deduplicated list of terms that should be highlighted
/// in search results. Terms are lowercased for case-insensitive matching.
///
/// # Example
/// ```ignore
/// // "report ext:pdf" -> ["pdf", "report"]
/// // "*test*.txt" -> [".txt", "test"]
/// // "foo bar" -> ["bar", "foo"]
/// ```
pub fn derive_highlight_terms(expr: &QueryExpression) -> Vec<String> {
    let mut collector = HighlightCollector::default();
    collector.collect_expr(expr);
    collector.into_terms()
}

#[derive(Default)]
struct HighlightCollector {
    terms: BTreeSet<String>,
}

impl HighlightCollector {
    fn collect_expr(&mut self, expr: &QueryExpression) {
        match expr {
            QueryExpression::Term(term) => self.collect_term(term),
            QueryExpression::Not(inner) => self.collect_expr(inner),
            QueryExpression::And(parts) | QueryExpression::Or(parts) => {
                for part in parts {
                    self.collect_expr(part);
                }
            }
        }
    }

    fn collect_term(&mut self, term: &QueryTerm) {
        match term {
            QueryTerm::Text(text) => self.collect_text(text),
            QueryTerm::Filter(filter) => self.collect_filter(filter),
        }
    }

    fn collect_filter(&mut self, filter: &QueryFilter) {
        match filter {
            // Extension filter - highlight the extension values
            QueryFilter::Extension(exts) => {
                for ext in exts {
                    self.push(ext.clone());
                }
            }
            // Type macro with argument - highlight the argument
            QueryFilter::TypeMacro {
                argument: Some(arg),
                ..
            } => {
                self.collect_text(arg);
            }
            // File/Folder with argument - highlight the argument
            QueryFilter::File {
                argument: Some(arg),
            }
            | QueryFilter::Folder {
                argument: Some(arg),
            } => {
                self.collect_text(arg);
            }
            // Path-based filters - extract filename from path
            QueryFilter::Parent { path }
            | QueryFilter::InFolder { path }
            | QueryFilter::NoSubfolders { path } => {
                // Extract just the last path component for highlighting
                if let Some(filename) = std::path::Path::new(path).file_name() {
                    if let Some(name) = filename.to_str() {
                        self.collect_text(name);
                    }
                }
            }
            // Content filter - highlight the search needle
            QueryFilter::Content { needle } => {
                self.collect_text(needle);
            }
            // Tag filter - highlight tag names
            QueryFilter::Tag { tags } => {
                for tag in tags {
                    self.push(tag.clone());
                }
            }
            // These filters don't contribute highlight terms
            QueryFilter::Type(_)
            | QueryFilter::TypeMacro { argument: None, .. }
            | QueryFilter::File { argument: None }
            | QueryFilter::Folder { argument: None }
            | QueryFilter::Size(_)
            | QueryFilter::DateModified(_)
            | QueryFilter::DateCreated(_) => {}
        }
    }

    fn collect_text(&mut self, value: &str) {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return;
        }

        // Split on wildcards to get literal chunks
        let chunks = literal_chunks(trimmed);
        for chunk in chunks {
            self.push(chunk);
        }
    }

    fn push(&mut self, candidate: String) {
        let lowercased = candidate.to_lowercase();
        if !lowercased.is_empty() {
            self.terms.insert(lowercased);
        }
    }

    fn into_terms(self) -> Vec<String> {
        self.terms.into_iter().collect()
    }
}

/// Splits a value on wildcards (* and ?) to get literal chunks.
///
/// Returns non-empty, trimmed chunks that can be used for highlighting.
fn literal_chunks(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let chunks: Vec<String> = trimmed
        .split(['*', '?'])
        .map(str::trim)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| chunk.to_string())
        .collect();

    // If no wildcards and we got nothing, return the whole string
    if chunks.is_empty() && !trimmed.contains(['*', '?']) {
        vec![trimmed.to_string()]
    } else {
        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{QueryFilter, TypeFilterTarget};

    fn text(s: &str) -> QueryExpression {
        QueryExpression::Term(QueryTerm::Text(s.to_string()))
    }

    fn ext(exts: &[&str]) -> QueryExpression {
        QueryExpression::Term(QueryTerm::Filter(QueryFilter::Extension(
            exts.iter().map(|s| s.to_string()).collect(),
        )))
    }

    fn tag(tags: &[&str]) -> QueryExpression {
        QueryExpression::Term(QueryTerm::Filter(QueryFilter::Tag {
            tags: tags.iter().map(|s| s.to_string()).collect(),
        }))
    }

    fn content(needle: &str) -> QueryExpression {
        QueryExpression::Term(QueryTerm::Filter(QueryFilter::Content {
            needle: needle.to_string(),
        }))
    }

    fn infolder(path: &str) -> QueryExpression {
        QueryExpression::Term(QueryTerm::Filter(QueryFilter::InFolder {
            path: path.to_string(),
        }))
    }

    // Basic text tests

    #[test]
    fn empty_query_returns_empty() {
        let expr = QueryExpression::And(vec![]);
        let terms = derive_highlight_terms(&expr);
        assert!(terms.is_empty());
    }

    #[test]
    fn single_word() {
        let terms = derive_highlight_terms(&text("report"));
        assert_eq!(terms, vec!["report"]);
    }

    #[test]
    fn single_word_uppercase_lowercased() {
        let terms = derive_highlight_terms(&text("REPORT"));
        assert_eq!(terms, vec!["report"]);
    }

    #[test]
    fn multiple_words_sorted() {
        let expr = QueryExpression::And(vec![text("foo"), text("bar"), text("baz")]);
        let terms = derive_highlight_terms(&expr);
        assert_eq!(terms, vec!["bar", "baz", "foo"]);
    }

    #[test]
    fn duplicate_words_deduplicated() {
        let expr = QueryExpression::And(vec![text("test"), text("TEST"), text("Test")]);
        let terms = derive_highlight_terms(&expr);
        assert_eq!(terms, vec!["test"]);
    }

    // Wildcard tests

    #[test]
    fn wildcard_star_splits() {
        let terms = derive_highlight_terms(&text("*test*"));
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn wildcard_question_splits() {
        let terms = derive_highlight_terms(&text("file?.txt"));
        assert_eq!(terms, vec![".txt", "file"]);
    }

    #[test]
    fn wildcard_multiple_segments() {
        let terms = derive_highlight_terms(&text("*hello*world*"));
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn wildcard_only_returns_empty() {
        let terms = derive_highlight_terms(&text("*"));
        assert!(terms.is_empty());
    }

    #[test]
    fn wildcard_extension_pattern() {
        let terms = derive_highlight_terms(&text("*.txt"));
        assert_eq!(terms, vec![".txt"]);
    }

    // Filter tests

    #[test]
    fn extension_filter() {
        let terms = derive_highlight_terms(&ext(&["pdf", "docx"]));
        assert_eq!(terms, vec!["docx", "pdf"]);
    }

    #[test]
    fn tag_filter() {
        let terms = derive_highlight_terms(&tag(&["Red", "Important"]));
        assert_eq!(terms, vec!["important", "red"]);
    }

    #[test]
    fn content_filter() {
        let terms = derive_highlight_terms(&content("search term"));
        assert_eq!(terms, vec!["search term"]);
    }

    #[test]
    fn infolder_extracts_filename() {
        let terms = derive_highlight_terms(&infolder("/Users/foo/Documents"));
        assert_eq!(terms, vec!["documents"]);
    }

    #[test]
    fn type_filter_no_highlights() {
        let expr =
            QueryExpression::Term(QueryTerm::Filter(QueryFilter::Type(TypeFilterTarget::File)));
        let terms = derive_highlight_terms(&expr);
        assert!(terms.is_empty());
    }

    // Boolean expression tests

    #[test]
    fn and_expression() {
        let expr = QueryExpression::And(vec![text("foo"), text("bar")]);
        let terms = derive_highlight_terms(&expr);
        assert_eq!(terms, vec!["bar", "foo"]);
    }

    #[test]
    fn or_expression() {
        let expr = QueryExpression::Or(vec![text("foo"), text("bar")]);
        let terms = derive_highlight_terms(&expr);
        assert_eq!(terms, vec!["bar", "foo"]);
    }

    #[test]
    fn not_expression_still_collects() {
        let expr = QueryExpression::Not(Box::new(text("excluded")));
        let terms = derive_highlight_terms(&expr);
        assert_eq!(terms, vec!["excluded"]);
    }

    #[test]
    fn nested_expressions() {
        let expr = QueryExpression::And(vec![
            text("outer"),
            QueryExpression::Or(vec![text("inner1"), text("inner2")]),
        ]);
        let terms = derive_highlight_terms(&expr);
        assert_eq!(terms, vec!["inner1", "inner2", "outer"]);
    }

    // Combined tests

    #[test]
    fn text_and_filter_combined() {
        let expr = QueryExpression::And(vec![text("report"), ext(&["pdf"])]);
        let terms = derive_highlight_terms(&expr);
        assert_eq!(terms, vec!["pdf", "report"]);
    }

    #[test]
    fn wildcard_and_filter_combined() {
        let expr = QueryExpression::And(vec![text("*test*.txt"), ext(&["rs"])]);
        let terms = derive_highlight_terms(&expr);
        assert_eq!(terms, vec![".txt", "rs", "test"]);
    }

    // Unicode tests

    #[test]
    fn unicode_text() {
        let terms = derive_highlight_terms(&text("你好世界"));
        assert_eq!(terms, vec!["你好世界"]);
    }

    #[test]
    fn emoji_text() {
        let terms = derive_highlight_terms(&text("test🔥file"));
        assert_eq!(terms, vec!["test🔥file"]);
    }
}
