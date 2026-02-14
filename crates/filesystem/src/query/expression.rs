//! Query expression types and AST nodes.

use super::date_filter::DatePredicate;
use super::size::SizePredicate;
use super::type_filter::TypeFilterTarget;

/// A parsed query expression (AST node).
#[derive(Debug, Clone)]
pub enum QueryExpression {
    Term(QueryTerm),
    Not(Box<QueryExpression>),
    And(Vec<QueryExpression>),
    Or(Vec<QueryExpression>),
}

/// A single query term (leaf node in the AST).
#[derive(Debug, Clone)]
pub enum QueryTerm {
    Text(String),
    Filter(QueryFilter),
}

/// A query filter (typed constraint).
#[derive(Debug, Clone)]
pub enum QueryFilter {
    Extension(Vec<String>),
    Type(TypeFilterTarget),
    TypeMacro {
        target: TypeFilterTarget,
        argument: Option<String>,
    },
    File {
        argument: Option<String>,
    },
    Folder {
        argument: Option<String>,
    },
    Parent {
        path: String,
    },
    InFolder {
        path: String,
    },
    NoSubfolders {
        path: String,
    },
    Size(SizePredicate),
    /// Content search filter - searches file contents using Rabin-Karp.
    Content {
        /// The needle to search for (already lowercased if case-insensitive).
        needle: String,
    },
    /// macOS Finder tag filter - matches files with specific tags.
    ///
    /// This is a macOS-specific feature that reads the
    /// `com.apple.metadata:_kMDItemUserTags` extended attribute.
    Tag {
        /// Tag names to match (OR semantics - matches any).
        tags: Vec<String>,
    },
    /// Date modified filter (`dm:` / `datemodified:`).
    DateModified(DatePredicate),
    /// Date created filter (`dc:` / `datecreated:`).
    DateCreated(DatePredicate),
}

/// Checks if an expression contains at least one concrete term.
pub fn query_expression_has_terms(expression: &QueryExpression) -> bool {
    match expression {
        QueryExpression::Term(_) => true,
        QueryExpression::Not(inner) => query_expression_has_terms(inner),
        QueryExpression::And(parts) | QueryExpression::Or(parts) => {
            parts.iter().any(query_expression_has_terms)
        }
    }
}

/// Converts the entire expression tree to lowercase for case-insensitive matching.
pub fn lowercase_query_expression(expression: QueryExpression) -> QueryExpression {
    match expression {
        QueryExpression::Term(term) => QueryExpression::Term(match term {
            QueryTerm::Text(value) => QueryTerm::Text(value.to_ascii_lowercase()),
            QueryTerm::Filter(filter) => QueryTerm::Filter(lowercase_query_filter(filter)),
        }),
        QueryExpression::Not(inner) => {
            QueryExpression::Not(Box::new(lowercase_query_expression(*inner)))
        }
        QueryExpression::And(parts) => QueryExpression::And(
            parts
                .into_iter()
                .map(lowercase_query_expression)
                .collect::<Vec<_>>(),
        ),
        QueryExpression::Or(parts) => QueryExpression::Or(
            parts
                .into_iter()
                .map(lowercase_query_expression)
                .collect::<Vec<_>>(),
        ),
    }
}

fn lowercase_query_filter(filter: QueryFilter) -> QueryFilter {
    match filter {
        QueryFilter::TypeMacro { target, argument } => QueryFilter::TypeMacro {
            target,
            argument: argument.map(|value| value.to_ascii_lowercase()),
        },
        QueryFilter::File { argument } => QueryFilter::File {
            argument: argument.map(|value| value.to_ascii_lowercase()),
        },
        QueryFilter::Folder { argument } => QueryFilter::Folder {
            argument: argument.map(|value| value.to_ascii_lowercase()),
        },
        QueryFilter::Parent { path } => QueryFilter::Parent {
            path: path.to_ascii_lowercase(),
        },
        QueryFilter::InFolder { path } => QueryFilter::InFolder {
            path: path.to_ascii_lowercase(),
        },
        QueryFilter::NoSubfolders { path } => QueryFilter::NoSubfolders {
            path: path.to_ascii_lowercase(),
        },
        QueryFilter::Content { needle } => QueryFilter::Content {
            needle: needle.to_ascii_lowercase(),
        },
        QueryFilter::Tag { tags } => QueryFilter::Tag {
            tags: tags.into_iter().map(|t| t.to_ascii_lowercase()).collect(),
        },
        // Date predicates don't need lowercasing
        QueryFilter::DateModified(_) | QueryFilter::DateCreated(_) => filter,
        other => other,
    }
}
