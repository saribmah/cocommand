use std::path::PathBuf;

use serde_json::json;

use crate::error::CoreError;

use super::types::EntryKindFilter;
use super::{
    canonicalize_existing_path, normalize_input_path, parse_ignore_paths,
    parse_search_request_options,
};

#[test]
fn search_request_options_default_to_hidden_and_unbounded_depth() {
    let input = json!({});
    let options = parse_search_request_options(&input).expect("options parse");

    assert!(
        options.include_hidden,
        "hidden files should be included by default"
    );
    assert_eq!(
        options.max_depth,
        usize::MAX,
        "default max depth should be unbounded"
    );
    assert_eq!(options.max_results, 50);
    assert!(!options.case_sensitive);
}

#[test]
fn search_request_options_honor_explicit_max_depth() {
    let input = json!({ "maxDepth": 3, "includeHidden": false });
    let options = parse_search_request_options(&input).expect("options parse");

    assert_eq!(options.max_depth, 3);
    assert!(!options.include_hidden);
}

#[test]
fn search_request_options_reject_non_integer_depth() {
    let input = json!({ "maxDepth": "deep" });
    let error = parse_search_request_options(&input).expect_err("depth should fail");

    match error {
        CoreError::InvalidInput(message) => {
            assert!(
                message.contains("maxDepth must be an integer"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected invalid input, got: {other:?}"),
    }
}

#[test]
fn parse_ignore_paths_supports_root_relative_and_deduped_values() {
    let root = PathBuf::from("/tmp/root");
    let input = json!({
        "ignorePaths": [
            "build",
            "/tmp/root/build",
            "nested/output"
        ]
    });

    let parsed = parse_ignore_paths(&input, &root, &[]).expect("ignore paths parse");
    assert_eq!(parsed.len(), 2);
    assert!(parsed.iter().any(|path| path == &root.join("build")));
    assert!(parsed
        .iter()
        .any(|path| path == &root.join("nested/output")));
}

#[test]
fn parse_ignore_paths_uses_defaults_when_omitted() {
    let root = PathBuf::from("/tmp/root");
    let input = json!({});
    let defaults = vec!["cache".to_string(), "tmp/output".to_string()];

    let parsed = parse_ignore_paths(&input, &root, &defaults).expect("ignore paths parse");
    assert_eq!(parsed.len(), 2);
    assert!(parsed.iter().any(|path| path == &root.join("cache")));
    assert!(parsed.iter().any(|path| path == &root.join("tmp/output")));
}

#[test]
fn normalize_input_path_resolves_relative_paths_from_workspace() {
    let workspace = PathBuf::from("/tmp/ws");
    let resolved = normalize_input_path("notes/todo.md", &workspace).expect("path resolves");
    assert_eq!(resolved, PathBuf::from("/tmp/ws/notes/todo.md"));
}

#[test]
fn entry_kind_filter_parse_valid_values() {
    assert_eq!(
        EntryKindFilter::parse(Some("all")).unwrap(),
        EntryKindFilter::All
    );
    assert_eq!(
        EntryKindFilter::parse(Some("file")).unwrap(),
        EntryKindFilter::File
    );
    assert_eq!(
        EntryKindFilter::parse(Some("directory")).unwrap(),
        EntryKindFilter::Directory
    );
    assert_eq!(
        EntryKindFilter::parse(None).unwrap(),
        EntryKindFilter::All,
        "default should be All"
    );
}

#[test]
fn entry_kind_filter_parse_invalid_value() {
    let error = EntryKindFilter::parse(Some("symlink")).expect_err("invalid kind should fail");
    match error {
        CoreError::InvalidInput(message) => {
            assert!(
                message.contains("unsupported kind"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected invalid input, got: {other:?}"),
    }
}

#[test]
fn canonicalize_existing_path_returns_original_for_nonexistent() {
    let path = PathBuf::from("/nonexistent/path/that/does/not/exist");
    let result = canonicalize_existing_path(path.clone());
    assert_eq!(
        result, path,
        "should return original path if canonicalization fails"
    );
}
