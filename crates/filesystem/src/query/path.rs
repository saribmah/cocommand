//! Path normalization and expansion utilities.

use std::path::{Path, PathBuf};

use crate::error::{FilesystemError, Result};

/// Normalizes a path for comparison (forward slashes, no trailing slash).
pub fn normalize_path_for_compare(raw: &str) -> String {
    let mut normalized = raw.replace('\\', "/");

    while normalized.ends_with('/') {
        if normalized == "/" || looks_like_windows_drive_root(normalized.as_str()) {
            break;
        }
        normalized.pop();
    }

    if normalized.is_empty() {
        "/".to_string()
    } else {
        normalized
    }
}

fn looks_like_windows_drive_root(path: &str) -> bool {
    path.len() == 3
        && path.as_bytes()[1] == b':'
        && path.as_bytes()[2] == b'/'
        && path.as_bytes()[0].is_ascii_alphabetic()
}

/// Splits a path into segments.
pub fn split_path_segments(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// Checks if a candidate path is a direct child of the parent path.
pub fn is_direct_child_path(candidate: &str, parent: &str) -> bool {
    if candidate == parent {
        return false;
    }
    let Some(candidate_parent) = Path::new(candidate).parent() else {
        return false;
    };
    normalize_path_for_compare(candidate_parent.to_string_lossy().as_ref()) == parent
}

/// Checks if a candidate path is a descendant of the parent path.
pub fn is_descendant_path(candidate: &str, parent: &str) -> bool {
    if candidate == parent {
        return false;
    }
    let prefix = format!("{parent}/");
    candidate.starts_with(prefix.as_str())
}

/// Normalizes a scope filter path (handles ~ expansion).
pub fn normalize_scope_filter_path(raw: &str, filter_name: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(FilesystemError::QueryParse(format!(
            "{filter_name}: requires a folder path"
        )));
    }

    let expanded = if trimmed == "~" || trimmed.starts_with("~/") || trimmed.starts_with("~\\") {
        expand_home_path(trimmed)?
    } else {
        PathBuf::from(trimmed)
    };
    Ok(normalize_path_for_compare(
        expanded.to_string_lossy().as_ref(),
    ))
}

fn expand_home_path(raw: &str) -> Result<PathBuf> {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| FilesystemError::Path("HOME is not set".to_string()))?;
    if raw == "~" {
        return Ok(home);
    }
    let rest = raw
        .strip_prefix("~/")
        .or_else(|| raw.strip_prefix("~\\"))
        .unwrap_or_default();
    Ok(home.join(rest))
}

/// Extracts the extension from a filename.
pub fn extension_of_name(name: &str) -> Option<String> {
    let split = name.rfind('.')?;
    if split + 1 >= name.len() {
        return None;
    }
    Some(name[split + 1..].to_ascii_lowercase())
}
