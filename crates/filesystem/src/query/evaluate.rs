//! Query evaluation logic for matching terms against nodes.

use super::context::NodeQueryContext;
use super::expression::{QueryFilter, QueryTerm};
use super::path::{is_descendant_path, is_direct_child_path};
use super::text_match::text_matches;
use super::type_filter::TypeFilterTarget;

/// Evaluates a query term against a NodeQueryContext.
pub fn evaluate_node_query_term(term: &QueryTerm, context: &NodeQueryContext) -> bool {
    match term {
        QueryTerm::Text(value) => text_matches_node(value, context),
        QueryTerm::Filter(filter) => evaluate_node_query_filter(filter, context),
    }
}

fn text_matches_node(value: &str, context: &NodeQueryContext) -> bool {
    text_matches(
        value,
        context.name(),
        context.path(),
        context.path_segments(),
    )
}

fn evaluate_node_query_filter(filter: &QueryFilter, context: &NodeQueryContext) -> bool {
    match filter {
        QueryFilter::Extension(extensions) => {
            if !context.is_file() {
                return false;
            }
            let Some(extension) = context.extension() else {
                return false;
            };
            extensions.iter().any(|candidate| candidate == extension)
        }
        QueryFilter::Type(target) => matches_node_type_filter_target(target, context),
        QueryFilter::TypeMacro { target, argument } => {
            if !matches_node_type_filter_target(target, context) {
                return false;
            }
            match argument.as_ref() {
                None => true,
                Some(value) => text_matches_node(value, context),
            }
        }
        QueryFilter::File { argument } => {
            if !context.is_file() {
                return false;
            }
            match argument.as_ref() {
                None => true,
                Some(value) => text_matches_node(value, context),
            }
        }
        QueryFilter::Folder { argument } => {
            if !context.is_dir() {
                return false;
            }
            match argument.as_ref() {
                None => true,
                Some(value) => text_matches_node(value, context),
            }
        }
        QueryFilter::Parent { path } => {
            is_direct_child_path(context.comparable_path(), path.as_str())
        }
        QueryFilter::InFolder { path } => {
            is_descendant_path(context.comparable_path(), path.as_str())
        }
        QueryFilter::NoSubfolders { path } => {
            context.comparable_path() == path.as_str()
                || (context.is_file()
                    && is_direct_child_path(context.comparable_path(), path.as_str()))
        }
        QueryFilter::Size(predicate) => {
            if !context.is_file() {
                return false;
            }
            let Some(size) = context.size() else {
                return false;
            };
            predicate.matches(size)
        }
        // Content filter is evaluated separately with parallel file I/O in search.rs
        QueryFilter::Content { .. } => false,
        // Tag filter is evaluated separately with parallel xattr reading in search.rs
        QueryFilter::Tag { .. } => false,
        // Date modified filter
        QueryFilter::DateModified(predicate) => {
            let Some(mtime) = context.modified_at() else {
                return false;
            };
            predicate.matches(mtime as i64)
        }
        // Date created filter
        QueryFilter::DateCreated(predicate) => {
            let Some(ctime) = context.created_at() else {
                return false;
            };
            predicate.matches(ctime as i64)
        }
    }
}

fn matches_node_type_filter_target(target: &TypeFilterTarget, context: &NodeQueryContext) -> bool {
    match target {
        TypeFilterTarget::File => context.is_file(),
        TypeFilterTarget::Directory => context.is_dir(),
        TypeFilterTarget::Extensions(extensions) => {
            if !context.is_file() {
                return false;
            }
            let Some(extension) = context.extension() else {
                return false;
            };
            extensions.iter().any(|candidate| *candidate == extension)
        }
    }
}
