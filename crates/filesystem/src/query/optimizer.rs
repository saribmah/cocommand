//! Query optimization for filesystem search.
//!
//! Applies deterministic rewrites that make downstream evaluation cheaper:
//! - Flattens nested AND/OR expressions
//! - Removes empty operands from conjunctions
//! - Reorders filters by cost (scope filters first, tag filters last)
//!
//! This follows Cardinal's `optimize_query` approach.

use super::expression::{QueryExpression, QueryFilter, QueryTerm};

/// Optimizes a query expression for efficient evaluation.
///
/// The optimizer applies the following transformations:
/// - Flattens nested AND expressions into a single AND
/// - Flattens nested OR expressions into a single OR
/// - Removes single-item AND/OR wrappers
/// - Reorders terms in AND expressions by evaluation cost
///
/// The function never mutates the input; a new tree is returned.
pub fn optimize_expression(expr: QueryExpression) -> QueryExpression {
    match expr {
        QueryExpression::And(parts) => optimize_and(parts),
        QueryExpression::Or(parts) => optimize_or(parts),
        QueryExpression::Not(inner) => QueryExpression::Not(Box::new(optimize_expression(*inner))),
        QueryExpression::Term(_) => expr,
    }
}

/// Normalizes AND expressions by flattening nested ANDs and reordering by priority.
fn optimize_and(parts: Vec<QueryExpression>) -> QueryExpression {
    let mut flattened = Vec::new();

    for expr in parts.into_iter().map(optimize_expression) {
        match expr {
            // Flatten nested ANDs
            QueryExpression::And(nested) => flattened.extend(nested),
            other => flattened.push(other),
        }
    }

    match flattened.len() {
        0 => {
            // Empty AND - should not happen in practice, return empty term
            QueryExpression::And(vec![])
        }
        1 => flattened.pop().unwrap(),
        _ => {
            reorder_by_priority(&mut flattened);
            QueryExpression::And(flattened)
        }
    }
}

/// Normalizes OR expressions by flattening nested ORs.
fn optimize_or(parts: Vec<QueryExpression>) -> QueryExpression {
    let mut flattened = Vec::new();

    for expr in parts.into_iter().map(optimize_expression) {
        match expr {
            // Flatten nested ORs
            QueryExpression::Or(nested) => flattened.extend(nested),
            other => flattened.push(other),
        }
    }

    match flattened.len() {
        0 => QueryExpression::Or(vec![]),
        1 => flattened.pop().unwrap(),
        _ => QueryExpression::Or(flattened),
    }
}

/// Reorders expression parts by priority to optimize query evaluation.
///
/// Priority levels (lower executes first):
/// - 0: Scope filters (`infolder:`, `parent:`) - narrow search space first
/// - 1: Non-filter terms (words, phrases, boolean ops) - cheap string matching
/// - 2: Generic filters (`ext:`, `type:`, `size:`, `content:`, etc.) - moderate cost
/// - 3: Tag filters (`tag:`) - expensive metadata access, runs last
fn reorder_by_priority(parts: &mut Vec<QueryExpression>) {
    if parts.len() <= 1 {
        return;
    }

    let priority = |expr: &QueryExpression| -> u8 {
        match expr {
            QueryExpression::Term(QueryTerm::Filter(filter)) => match filter {
                // Scope filters - narrow search space first
                QueryFilter::InFolder { .. } | QueryFilter::Parent { .. } => 0,
                // Tag filters - expensive metadata access
                QueryFilter::Tag { .. } => 3,
                // All other filters - moderate cost
                _ => 2,
            },
            // Non-filter terms - cheap string matching
            _ => 1,
        }
    };

    let mut keyed: Vec<_> = parts
        .drain(..)
        .map(|expr| (priority(&expr), expr))
        .collect();
    keyed.sort_by_key(|(prio, _)| *prio);
    parts.extend(keyed.into_iter().map(|(_, expr)| expr));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::QueryFilter;

    fn text(s: &str) -> QueryExpression {
        QueryExpression::Term(QueryTerm::Text(s.to_string()))
    }

    fn ext(e: &str) -> QueryExpression {
        QueryExpression::Term(QueryTerm::Filter(QueryFilter::Extension(vec![
            e.to_string()
        ])))
    }

    fn infolder(p: &str) -> QueryExpression {
        QueryExpression::Term(QueryTerm::Filter(QueryFilter::InFolder {
            path: p.to_string(),
        }))
    }

    fn tag(t: &str) -> QueryExpression {
        QueryExpression::Term(QueryTerm::Filter(QueryFilter::Tag {
            tags: vec![t.to_string()],
        }))
    }

    fn content(c: &str) -> QueryExpression {
        QueryExpression::Term(QueryTerm::Filter(QueryFilter::Content {
            needle: c.to_string(),
        }))
    }

    #[test]
    fn flatten_nested_and() {
        // (a AND (b AND c)) -> (a AND b AND c)
        let expr = QueryExpression::And(vec![
            text("a"),
            QueryExpression::And(vec![text("b"), text("c")]),
        ]);
        let optimized = optimize_expression(expr);

        match optimized {
            QueryExpression::And(parts) => {
                assert_eq!(parts.len(), 3);
            }
            _ => panic!("Expected And"),
        }
    }

    #[test]
    fn flatten_nested_or() {
        // (a OR (b OR c)) -> (a OR b OR c)
        let expr = QueryExpression::Or(vec![
            text("a"),
            QueryExpression::Or(vec![text("b"), text("c")]),
        ]);
        let optimized = optimize_expression(expr);

        match optimized {
            QueryExpression::Or(parts) => {
                assert_eq!(parts.len(), 3);
            }
            _ => panic!("Expected Or"),
        }
    }

    #[test]
    fn unwrap_single_item_and() {
        let expr = QueryExpression::And(vec![text("alone")]);
        let optimized = optimize_expression(expr);

        match optimized {
            QueryExpression::Term(QueryTerm::Text(s)) => {
                assert_eq!(s, "alone");
            }
            _ => panic!("Expected single term"),
        }
    }

    #[test]
    fn unwrap_single_item_or() {
        let expr = QueryExpression::Or(vec![text("alone")]);
        let optimized = optimize_expression(expr);

        match optimized {
            QueryExpression::Term(QueryTerm::Text(s)) => {
                assert_eq!(s, "alone");
            }
            _ => panic!("Expected single term"),
        }
    }

    #[test]
    fn reorder_scope_filters_first() {
        // ext:txt infolder:/foo -> infolder:/foo ext:txt
        let expr = QueryExpression::And(vec![ext("txt"), infolder("/foo")]);
        let optimized = optimize_expression(expr);

        match optimized {
            QueryExpression::And(parts) => {
                assert_eq!(parts.len(), 2);
                // infolder should come first (priority 0)
                assert!(matches!(
                    &parts[0],
                    QueryExpression::Term(QueryTerm::Filter(QueryFilter::InFolder { .. }))
                ));
                // ext should come second (priority 2)
                assert!(matches!(
                    &parts[1],
                    QueryExpression::Term(QueryTerm::Filter(QueryFilter::Extension(_)))
                ));
            }
            _ => panic!("Expected And"),
        }
    }

    #[test]
    fn reorder_tag_filters_last() {
        // tag:red ext:txt -> ext:txt tag:red
        let expr = QueryExpression::And(vec![tag("red"), ext("txt")]);
        let optimized = optimize_expression(expr);

        match optimized {
            QueryExpression::And(parts) => {
                assert_eq!(parts.len(), 2);
                // ext should come first (priority 2)
                assert!(matches!(
                    &parts[1],
                    QueryExpression::Term(QueryTerm::Filter(QueryFilter::Tag { .. }))
                ));
            }
            _ => panic!("Expected And"),
        }
    }

    #[test]
    fn reorder_content_before_tag() {
        // tag:red content:foo -> content:foo tag:red
        let expr = QueryExpression::And(vec![tag("red"), content("foo")]);
        let optimized = optimize_expression(expr);

        match optimized {
            QueryExpression::And(parts) => {
                assert_eq!(parts.len(), 2);
                // content (priority 2) before tag (priority 3)
                assert!(matches!(
                    &parts[0],
                    QueryExpression::Term(QueryTerm::Filter(QueryFilter::Content { .. }))
                ));
                assert!(matches!(
                    &parts[1],
                    QueryExpression::Term(QueryTerm::Filter(QueryFilter::Tag { .. }))
                ));
            }
            _ => panic!("Expected And"),
        }
    }

    #[test]
    fn reorder_text_between_scope_and_filters() {
        // ext:txt report infolder:/foo -> infolder:/foo report ext:txt
        let expr = QueryExpression::And(vec![ext("txt"), text("report"), infolder("/foo")]);
        let optimized = optimize_expression(expr);

        match optimized {
            QueryExpression::And(parts) => {
                assert_eq!(parts.len(), 3);
                // infolder (0), text (1), ext (2)
                assert!(matches!(
                    &parts[0],
                    QueryExpression::Term(QueryTerm::Filter(QueryFilter::InFolder { .. }))
                ));
                assert!(matches!(
                    &parts[1],
                    QueryExpression::Term(QueryTerm::Text(_))
                ));
                assert!(matches!(
                    &parts[2],
                    QueryExpression::Term(QueryTerm::Filter(QueryFilter::Extension(_)))
                ));
            }
            _ => panic!("Expected And"),
        }
    }

    #[test]
    fn optimize_deeply_nested() {
        // ((a AND b) AND (c AND d)) -> (a AND b AND c AND d)
        let expr = QueryExpression::And(vec![
            QueryExpression::And(vec![text("a"), text("b")]),
            QueryExpression::And(vec![text("c"), text("d")]),
        ]);
        let optimized = optimize_expression(expr);

        match optimized {
            QueryExpression::And(parts) => {
                assert_eq!(parts.len(), 4);
            }
            _ => panic!("Expected And"),
        }
    }

    #[test]
    fn optimize_not_expression() {
        // NOT (a AND (b AND c)) -> NOT (a AND b AND c)
        let inner = QueryExpression::And(vec![
            text("a"),
            QueryExpression::And(vec![text("b"), text("c")]),
        ]);
        let expr = QueryExpression::Not(Box::new(inner));
        let optimized = optimize_expression(expr);

        match optimized {
            QueryExpression::Not(inner) => match *inner {
                QueryExpression::And(parts) => {
                    assert_eq!(parts.len(), 3);
                }
                _ => panic!("Expected And inside Not"),
            },
            _ => panic!("Expected Not"),
        }
    }
}
