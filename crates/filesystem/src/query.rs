//! Query parsing, compilation, and matching for filesystem search.
//!
//! This module provides the query language for filesystem search, including:
//! - Expression types (AND, OR, NOT, terms)
//! - Filter types (extension, type, size, folder scope, content, date)
//! - Query parsing and tokenization
//! - Query optimization (flattening, reordering by cost)
//! - Matching against SlabNode

mod content;
mod context;
mod date_filter;
mod evaluate;
mod expression;
mod highlight;
mod matcher;
mod optimizer;
mod parser;
mod path;
mod size;
mod text_match;
mod type_filter;

// Re-export public types
pub use content::file_content_matches;
pub use expression::{QueryExpression, QueryFilter, QueryTerm};
pub use matcher::SearchQueryMatcher;
pub use parser::QueryParser;
pub use type_filter::TypeFilterTarget;

// Internal re-exports for index module (will be used when index module is added)
#[allow(unused_imports)]
pub(crate) use context::NodeQueryContext;
#[allow(unused_imports)]
pub(crate) use evaluate::evaluate_node_query_term;
#[allow(unused_imports)]
pub(crate) use path::normalize_path_for_compare;
