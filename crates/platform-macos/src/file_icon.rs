//! File icon extraction for macOS.
//!
//! This module provides functionality to extract file icons using the `icns` crate.
//! For .app bundles, it reads the icon directly from the app's icns file.
//!
//! ## Features
//!
//! - Extract app icons from .icns files in app bundles
//! - Returns PNG data as base64-encoded data URI
//!
//! ## Example
//!
//! ```ignore
//! use platform_macos::file_icon::icon_of_path;
//!
//! if let Some(data_uri) = icon_of_path("/Applications/Safari.app") {
//!     // data_uri is "data:image/png;base64,..."
//! }
//! ```

use base64::{engine::general_purpose::STANDARD, Engine};
use icns::{IconFamily, IconType};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Icon types to try, in order of preference (targeting ~32x32 size)
const ICON_TYPES_TO_TRY: &[IconType] = &[
    IconType::RGBA32_32x32,
    IconType::RGBA32_16x16_2x, // 32x32 retina
    IconType::RGB24_32x32,
    IconType::RGBA32_16x16,
    IconType::RGB24_16x16,
    IconType::RGBA32_64x64,
    IconType::RGB24_48x48,
    IconType::RGBA32_128x128,
    IconType::RGB24_128x128,
];

/// Extracts the icon for an app bundle and returns it as a base64 data URI.
///
/// For .app bundles, reads the icon file specified in Info.plist and extracts
/// it using the `icns` crate.
///
/// ## Arguments
///
/// * `path` - The path to the .app bundle
///
/// ## Returns
///
/// A base64-encoded PNG data URI (e.g., "data:image/png;base64,...") or `None` if
/// icon extraction fails.
pub fn icon_of_path(path: &str) -> Option<String> {
    let png_data = icon_of_path_raw(path)?;
    let base64_data = STANDARD.encode(&png_data);
    Some(format!("data:image/png;base64,{}", base64_data))
}

/// Extracts the icon for an app bundle and returns raw PNG bytes.
///
/// This is the lower-level API that returns raw PNG data without base64 encoding.
pub fn icon_of_path_raw(path: &str) -> Option<Vec<u8>> {
    let app_path = Path::new(path);

    // Only handle .app bundles
    if app_path.extension().and_then(|e| e.to_str()) != Some("app") {
        return None;
    }

    // Read Info.plist to find the icon file name
    let plist_path = app_path.join("Contents/Info.plist");
    let plist = plist::Value::from_file(&plist_path).ok()?;
    let dict = plist.as_dictionary()?;

    // Get icon file name from CFBundleIconFile or CFBundleIconName
    let icon_name = dict
        .get("CFBundleIconFile")
        .or_else(|| dict.get("CFBundleIconName"))
        .and_then(|v| v.as_string())?;

    // Build path to the icon file
    let resources_path = app_path.join("Contents/Resources");

    // Try with .icns extension first, then without
    let icon_path = if icon_name.ends_with(".icns") {
        resources_path.join(icon_name)
    } else {
        let with_ext = resources_path.join(format!("{}.icns", icon_name));
        if with_ext.exists() {
            with_ext
        } else {
            resources_path.join(icon_name)
        }
    };

    if !icon_path.exists() {
        return None;
    }

    extract_png_from_icns(&icon_path)
}

/// Extract a PNG from an icns file using the icns crate
fn extract_png_from_icns(icns_path: &Path) -> Option<Vec<u8>> {
    let file = File::open(icns_path).ok()?;
    let reader = BufReader::new(file);
    let icon_family = IconFamily::read(reader).ok()?;

    // Try each icon type in order of preference
    for icon_type in ICON_TYPES_TO_TRY {
        if let Ok(image) = icon_family.get_icon_with_type(*icon_type) {
            let mut png_data = Vec::new();
            if image.write_png(&mut png_data).is_ok() && !png_data.is_empty() {
                return Some(png_data);
            }
        }
    }

    // If none of the preferred types worked, try to get any available icon
    for icon_type in icon_family.available_icons() {
        if let Ok(image) = icon_family.get_icon_with_type(icon_type) {
            let mut png_data = Vec::new();
            if image.write_png(&mut png_data).is_ok() && !png_data.is_empty() {
                return Some(png_data);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_of_existing_app() {
        // Test with Finder.app which should always exist
        let result = icon_of_path("/System/Library/CoreServices/Finder.app");
        assert!(result.is_some(), "should get icon for Finder.app");

        let data_uri = result.unwrap();
        assert!(
            data_uri.starts_with("data:image/png;base64,"),
            "should be a PNG data URI"
        );
    }

    #[test]
    fn icon_of_safari() {
        // Test with Safari.app
        let result = icon_of_path("/Applications/Safari.app");
        assert!(result.is_some(), "should get icon for Safari.app");
    }

    #[test]
    fn icon_of_nonexistent_app() {
        let result = icon_of_path("/nonexistent/path/file.app");
        assert!(result.is_none(), "should return None for nonexistent app");
    }

    #[test]
    fn icon_of_non_app_path() {
        // Should return None for non-.app paths
        let result = icon_of_path("/Applications");
        assert!(result.is_none(), "should return None for non-.app path");
    }

    #[test]
    fn icon_raw_returns_valid_png() {
        let result = icon_of_path_raw("/System/Library/CoreServices/Finder.app");
        assert!(result.is_some(), "should get raw PNG data");

        let data = result.unwrap();
        // PNG magic bytes
        assert!(
            data.len() > 8 && data[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            "should be valid PNG data"
        );
    }
}
