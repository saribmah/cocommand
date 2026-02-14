//! Query matcher for search operations.

use std::collections::BTreeSet;

use crate::error::Result;
use crate::storage::SlabNode;

use super::context::NodeQueryContext;
use super::evaluate::evaluate_node_query_term;
use super::expression::{lowercase_query_expression, QueryExpression, QueryTerm};
use super::highlight::derive_highlight_terms;
use super::optimizer::optimize_expression;
use super::parser::QueryParser;
use super::text_match::is_name_prefilter_term;

/// A compiled query matcher for search operations.
#[derive(Debug, Clone)]
pub struct SearchQueryMatcher {
    expression: QueryExpression,
    case_sensitive: bool,
}

impl SearchQueryMatcher {
    /// Compiles a raw query string into a matcher.
    ///
    /// The compilation pipeline:
    /// 1. Parse the raw query string into an AST
    /// 2. Lowercase for case-insensitive matching (if needed)
    /// 3. Optimize the expression (flatten nested AND/OR, reorder by cost)
    pub fn compile(raw_query: &str, case_sensitive: bool) -> Result<Self> {
        let parsed = QueryParser::parse(raw_query)?;
        let lowercased = if case_sensitive {
            parsed
        } else {
            lowercase_query_expression(parsed)
        };
        let expression = optimize_expression(lowercased);

        Ok(Self {
            expression,
            case_sensitive,
        })
    }

    /// Returns required name terms for prefiltering.
    pub fn required_name_terms(&self) -> Vec<String> {
        required_name_terms_from_expression(&self.expression)
            .into_iter()
            .collect()
    }

    /// Returns the parsed expression.
    pub fn expression(&self) -> &QueryExpression {
        &self.expression
    }

    /// Returns whether the matcher is case sensitive.
    pub fn case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    /// Returns terms that should be highlighted in search results.
    ///
    /// Terms are sorted, deduplicated, and lowercased for case-insensitive matching.
    /// Wildcards are split into literal chunks.
    pub fn highlight_terms(&self) -> Vec<String> {
        derive_highlight_terms(&self.expression)
    }

    /// Matches a query term against a SlabNode with its computed path.
    pub fn matches_node_term(&self, term: &QueryTerm, node: &SlabNode, path: &str) -> bool {
        let context = NodeQueryContext::new(node, path.to_string(), self.case_sensitive);
        evaluate_node_query_term(term, &context)
    }
}

fn required_name_terms_from_expression(expression: &QueryExpression) -> BTreeSet<String> {
    match expression {
        QueryExpression::Term(QueryTerm::Text(value)) => {
            if is_name_prefilter_term(value.as_str()) {
                std::iter::once(value.clone()).collect()
            } else {
                BTreeSet::new()
            }
        }
        QueryExpression::Term(QueryTerm::Filter(_)) => BTreeSet::new(),
        QueryExpression::Not(_) => BTreeSet::new(),
        QueryExpression::And(parts) => parts.iter().fold(BTreeSet::new(), |mut acc, part| {
            acc.extend(required_name_terms_from_expression(part));
            acc
        }),
        QueryExpression::Or(parts) => {
            let Some((first, rest)) = parts.split_first() else {
                return BTreeSet::new();
            };
            let mut required = required_name_terms_from_expression(first);
            for part in rest {
                let other = required_name_terms_from_expression(part);
                required = required
                    .intersection(&other)
                    .cloned()
                    .collect::<BTreeSet<_>>();
            }
            required
        }
    }
}
