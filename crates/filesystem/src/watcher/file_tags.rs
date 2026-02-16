//! macOS Finder tags support.
//!
//! This module provides functionality to read and search for macOS Finder tags
//! (user tags).
//!
//! ## Features
//!
//! - Read tags from files using the `com.apple.metadata:_kMDItemUserTags` xattr
//! - Search for files with specific tags using Spotlight's `mdfind` command
//! - Parse the binary plist format used by macOS for tag storage
//!
//! ## Platform Support
//!
//! This module is only available on macOS. On other platforms, the functions
//! return empty results or no-op.

use std::path::Path;

/// The extended attribute name for Finder user tags.
#[cfg(target_os = "macos")]
const USER_TAG_XATTR: &str = "com.apple.metadata:_kMDItemUserTags";

/// Reads Finder-style user tags from an on-disk item.
///
/// Returns a vector of tag names. If the file doesn't have tags, doesn't exist,
/// or if there's an error reading, returns an empty vector.
///
/// ## Arguments
///
/// * `path` - The path to read tags from
/// * `case_insensitive` - If true, tag names are lowercased
///
/// ## Example
///
/// ```ignore
/// let tags = read_tags_from_path(Path::new("/path/to/file.txt"), false);
/// if tags.contains(&"Important".to_string()) {
///     println!("File is tagged as Important");
/// }
/// ```
#[cfg(target_os = "macos")]
pub fn read_tags_from_path(path: &Path, case_insensitive: bool) -> Vec<String> {
    let raw = match xattr::get(path, USER_TAG_XATTR) {
        Ok(Some(data)) => data,
        Ok(None) | Err(_) => return Vec::new(),
    };
    parse_tags(&raw, case_insensitive)
}

/// Stub implementation for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn read_tags_from_path(_path: &Path, _case_insensitive: bool) -> Vec<String> {
    Vec::new()
}

/// Parses raw tag data from the binary plist format.
///
/// macOS stores tags as a binary plist array of strings. Each string may have
/// a suffix like `\n0` or `\n1` indicating the tag color, which is stripped.
#[cfg(target_os = "macos")]
pub fn parse_tags(raw: &[u8], case_insensitive: bool) -> Vec<String> {
    use plist::Value;
    use std::io::Cursor;

    let Ok(Value::Array(items)) = Value::from_reader(Cursor::new(raw)) else {
        return Vec::new();
    };

    items
        .into_iter()
        .filter_map(|value| match value {
            Value::String(text) => Some(strip_tag_suffix(&text, case_insensitive)),
            _ => None,
        })
        .collect()
}

/// Stub implementation for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn parse_tags(_raw: &[u8], _case_insensitive: bool) -> Vec<String> {
    Vec::new()
}

/// Strips the color suffix from a tag name and optionally lowercases it.
///
/// Tags are stored as `"TagName\n0"` where the number after the newline
/// indicates the tag color. This function extracts just the name portion.
pub fn strip_tag_suffix(value: &str, case_insensitive: bool) -> String {
    let name = value.split('\n').next().unwrap_or(value);
    if case_insensitive {
        name.to_ascii_lowercase()
    } else {
        name.to_string()
    }
}

/// Searches for files with the specified tags using the `mdfind` command-line tool.
///
/// This uses macOS Spotlight to find files with matching tags. It's more efficient
/// for searching across the filesystem than reading each file's xattr individually.
///
/// ## Arguments
///
/// * `tags` - The tags to search for (OR semantics - matches any)
/// * `case_insensitive` - Whether to do case-insensitive matching
///
/// ## Returns
///
/// A list of file paths that have any of the specified tags, or an error if
/// the mdfind command fails or if tags contain forbidden characters.
///
/// ## Forbidden Characters
///
/// Tags containing `'`, `\`, or `*` are rejected because they have special
/// meaning in Spotlight queries and could cause injection issues.
#[cfg(target_os = "macos")]
pub fn search_tags_using_mdfind(
    tags: Vec<String>,
    case_insensitive: bool,
) -> std::io::Result<Vec<std::path::PathBuf>> {
    use std::process::Command;

    if tags.is_empty() {
        return Ok(Vec::new());
    }

    // Check for forbidden characters
    for tag in &tags {
        if let Some(forbidden_char) = tag_has_spotlight_forbidden_chars(tag) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("tag filter contains unsupported character '{forbidden_char}': {tag}"),
            ));
        }
    }

    // Build the Spotlight query
    let modifier = if case_insensitive { "c" } else { "" };
    let query = tags
        .into_iter()
        .map(|tag| format!("kMDItemUserTags == '*{tag}*'{modifier}"))
        .collect::<Vec<_>>()
        .join(" || ");

    let output = Command::new("mdfind").arg(&query).output()?;

    if !output.status.success() {
        return Err(std::io::Error::other("mdfind command failed"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let paths = stdout.lines().map(std::path::PathBuf::from).collect();

    Ok(paths)
}

/// Stub implementation for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn search_tags_using_mdfind(
    _tags: Vec<String>,
    _case_insensitive: bool,
) -> std::io::Result<Vec<std::path::PathBuf>> {
    Ok(Vec::new())
}

/// Checks if a tag contains characters that are forbidden in Spotlight queries.
///
/// Returns the first forbidden character found, or None if the tag is safe.
pub fn tag_has_spotlight_forbidden_chars(tag: &str) -> Option<char> {
    tag.chars().find(|c| matches!(c, '\'' | '\\' | '*'))
}

/// Checks if a file has any of the specified tags.
///
/// This is a convenience function for filtering search results by tags.
///
/// ## Arguments
///
/// * `path` - The file to check
/// * `tags` - The tags to look for (case-insensitive matching)
/// * `case_insensitive` - Whether to do case-insensitive matching
///
/// ## Returns
///
/// `true` if the file has at least one of the specified tags.
pub fn file_has_any_tag(path: &Path, tags: &[String], case_insensitive: bool) -> bool {
    let file_tags = read_tags_from_path(path, case_insensitive);
    if file_tags.is_empty() || tags.is_empty() {
        return false;
    }

    if case_insensitive {
        let tags_lower: Vec<String> = tags.iter().map(|t| t.to_ascii_lowercase()).collect();
        file_tags
            .iter()
            .any(|ft| tags_lower.iter().any(|t| ft == t))
    } else {
        file_tags.iter().any(|ft| tags.contains(ft))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_tag_suffix_basic() {
        assert_eq!(strip_tag_suffix("Important", false), "Important");
        assert_eq!(strip_tag_suffix("Important\n0", false), "Important");
        assert_eq!(strip_tag_suffix("Archive\n1", false), "Archive");
    }

    #[test]
    fn strip_tag_suffix_case_insensitive() {
        assert_eq!(strip_tag_suffix("Important\n0", true), "important");
        assert_eq!(strip_tag_suffix("ARCHIVE", true), "archive");
    }

    #[test]
    fn strip_tag_suffix_empty() {
        assert_eq!(strip_tag_suffix("", false), "");
        assert_eq!(strip_tag_suffix("", true), "");
    }

    #[test]
    fn strip_tag_suffix_unicode() {
        assert_eq!(strip_tag_suffix("项目\n0", false), "项目");
        assert_eq!(strip_tag_suffix("Important\n0", false), "Important");
    }

    #[test]
    fn tag_forbidden_chars_safe() {
        assert_eq!(tag_has_spotlight_forbidden_chars("Project"), None);
        assert_eq!(tag_has_spotlight_forbidden_chars("Project-Alpha"), None);
        assert_eq!(tag_has_spotlight_forbidden_chars("Project_Beta"), None);
        assert_eq!(tag_has_spotlight_forbidden_chars("项目"), None);
    }

    #[test]
    fn tag_forbidden_chars_detected() {
        assert_eq!(
            tag_has_spotlight_forbidden_chars("Project'Alpha"),
            Some('\'')
        );
        assert_eq!(
            tag_has_spotlight_forbidden_chars("Project\\Beta"),
            Some('\\')
        );
        assert_eq!(tag_has_spotlight_forbidden_chars("Project*"), Some('*'));
    }

    #[test]
    fn tag_forbidden_chars_first_occurrence() {
        assert_eq!(tag_has_spotlight_forbidden_chars("A'B*C\\D"), Some('\''));
    }

    #[test]
    fn read_tags_nonexistent_file() {
        let tags = read_tags_from_path(Path::new("/nonexistent/path/file.txt"), false);
        assert!(tags.is_empty());
    }

    #[cfg(target_os = "macos")]
    mod macos_tests {
        use super::*;
        use plist::{to_writer_binary, Value};
        use tempfile::NamedTempFile;

        fn plist_bytes(values: &[Value]) -> Vec<u8> {
            let mut data = Vec::new();
            to_writer_binary(&mut data, &Value::Array(values.to_vec())).expect("serialize tags");
            data
        }

        fn write_xattr(path: &std::path::Path, tags: &[&str]) {
            let plist_values: Vec<Value> = tags
                .iter()
                .map(|tag| Value::String(format!("{tag}\n0")))
                .collect();
            let data = plist_bytes(&plist_values);
            xattr::set(path, USER_TAG_XATTR, &data).expect("write tag xattr");
        }

        #[test]
        fn parse_tags_valid_plist() {
            let bytes = plist_bytes(&[
                Value::String("Important\n0".into()),
                Value::String("Archive".into()),
            ]);
            let tags = parse_tags(&bytes, false);
            assert_eq!(tags, vec!["Important".to_string(), "Archive".to_string()]);
        }

        #[test]
        fn parse_tags_invalid_plist() {
            let bytes = b"not a plist";
            assert!(parse_tags(bytes, false).is_empty());
        }

        #[test]
        fn parse_tags_case_insensitive() {
            let bytes = plist_bytes(&[Value::String("Important\n0".into())]);
            let tags = parse_tags(&bytes, true);
            assert_eq!(tags, vec!["important".to_string()]);
        }

        #[test]
        fn parse_tags_ignores_non_strings() {
            let bytes = plist_bytes(&[
                Value::String("Project\n0".into()),
                Value::Integer(42.into()),
                Value::Boolean(true),
            ]);
            let tags = parse_tags(&bytes, false);
            assert_eq!(tags, vec!["Project".to_string()]);
        }

        #[test]
        fn read_tags_from_path_works() {
            let file = NamedTempFile::new().expect("create temp file");
            write_xattr(file.path(), &["Important", "Archive"]);

            let tags = read_tags_from_path(file.path(), false);
            assert_eq!(tags, vec!["Important".to_string(), "Archive".to_string()]);
        }

        #[test]
        fn read_tags_from_path_no_tags() {
            let file = NamedTempFile::new().expect("create temp file");
            let tags = read_tags_from_path(file.path(), false);
            assert!(tags.is_empty());
        }

        #[test]
        fn file_has_any_tag_matches() {
            let file = NamedTempFile::new().expect("create temp file");
            write_xattr(file.path(), &["Important", "Archive"]);

            assert!(file_has_any_tag(
                file.path(),
                &["Important".to_string()],
                false
            ));
            assert!(file_has_any_tag(
                file.path(),
                &["Archive".to_string()],
                false
            ));
            assert!(file_has_any_tag(
                file.path(),
                &["Other".to_string(), "Important".to_string()],
                false
            ));
        }

        #[test]
        fn file_has_any_tag_no_match() {
            let file = NamedTempFile::new().expect("create temp file");
            write_xattr(file.path(), &["Important"]);

            assert!(!file_has_any_tag(
                file.path(),
                &["Archive".to_string()],
                false
            ));
        }

        #[test]
        fn file_has_any_tag_case_insensitive() {
            let file = NamedTempFile::new().expect("create temp file");
            write_xattr(file.path(), &["Important"]);

            assert!(file_has_any_tag(
                file.path(),
                &["important".to_string()],
                true
            ));
            assert!(file_has_any_tag(
                file.path(),
                &["IMPORTANT".to_string()],
                true
            ));
        }

        #[test]
        fn search_tags_mdfind_empty() {
            let result = search_tags_using_mdfind(vec![], false);
            assert!(result.is_ok());
            assert!(result.unwrap().is_empty());
        }

        #[test]
        fn search_tags_mdfind_rejects_forbidden() {
            let result = search_tags_using_mdfind(vec!["Project'Alpha".to_string()], false);
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
        }
    }
}
