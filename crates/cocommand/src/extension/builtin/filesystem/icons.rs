//! File icon extraction for the filesystem extension.
//!
//! This module provides a platform-abstracted interface for extracting file icons.
//! On macOS, it uses NSWorkspace to get system icons.
//! On other platforms, it returns None (icons not supported).

use serde::Serialize;

/// Result of icon extraction for multiple paths.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IconsPayload {
    /// The icons for each requested path, in the same order.
    pub icons: Vec<IconResult>,
}

/// Result of icon extraction for a single path.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IconResult {
    /// The path that was requested.
    pub path: String,
    /// Base64-encoded PNG data URI, or None if icon extraction failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// Extracts icons for a list of file paths.
///
/// Returns icons in the same order as the input paths.
/// Uses rayon for parallel icon extraction.
#[cfg(target_os = "macos")]
pub fn extract_icons(paths: Vec<String>) -> IconsPayload {
    use rayon::prelude::*;

    let icons: Vec<IconResult> = paths
        .into_par_iter()
        .map(|path| {
            let icon = platform_macos::icon_of_path(&path);
            IconResult { path, icon }
        })
        .collect();

    IconsPayload { icons }
}

/// Stub implementation for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn extract_icons(paths: Vec<String>) -> IconsPayload {
    let icons = paths
        .into_iter()
        .map(|path| IconResult { path, icon: None })
        .collect();

    IconsPayload { icons }
}
