//! File icon extraction for macOS.
//!
//! This module provides functionality to extract file icons using macOS system APIs.
//! It uses NSWorkspace to get system icons for files and directories.
//!
//! ## Features
//!
//! - Extract system icons via `NSWorkspace::iconForFile`
//! - Returns PNG data as base64-encoded data URI
//! - Icons are scaled to 32x32 pixels
//!
//! ## Example
//!
//! ```ignore
//! use platform_macos::file_icon::icon_of_path;
//!
//! if let Some(data_uri) = icon_of_path("/path/to/file.txt") {
//!     // data_uri is "data:image/png;base64,..."
//! }
//! ```

use base64::{engine::general_purpose::STANDARD, Engine};
use objc2::rc::autoreleasepool;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use objc2_foundation::{CGPoint, CGRect, CGSize, NSString};
use std::ffi::c_void;

/// Target icon size in pixels.
const ICON_SIZE: f64 = 32.0;

/// PNG file type constant for NSBitmapImageRep.
const NS_PNG_FILE_TYPE: usize = 4;

/// Extracts the system icon for a file path and returns it as a base64 data URI.
///
/// Uses `NSWorkspace::iconForFile` to get the system icon, then converts it to PNG.
///
/// ## Arguments
///
/// * `path` - The file path to get the icon for
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

/// Extracts the system icon for a file path and returns raw PNG bytes.
///
/// This is the lower-level API that returns raw PNG data without base64 encoding.
pub fn icon_of_path_raw(path: &str) -> Option<Vec<u8>> {
    autoreleasepool(|_| unsafe {
        // Get NSWorkspace
        let workspace: *mut AnyObject = msg_send![class!(NSWorkspace), sharedWorkspace];
        if workspace.is_null() {
            return None;
        }

        // Create NSString from path
        let path_ns = NSString::from_str(path);

        // Get icon for file
        let image: *mut AnyObject = msg_send![workspace, iconForFile: &*path_ns];
        if image.is_null() {
            return None;
        }

        // Set the icon size
        let size = CGSize::new(ICON_SIZE, ICON_SIZE);
        let _: () = msg_send![image, setSize: size];

        // Lock focus on the image to draw it
        let locked: bool = msg_send![image, lockFocus];
        if !locked {
            return None;
        }

        // Get the bitmap representation
        let bitmap: *mut AnyObject = msg_send![class!(NSBitmapImageRep), alloc];
        if bitmap.is_null() {
            let _: () = msg_send![image, unlockFocus];
            return None;
        }

        // Initialize with focused view rect
        let rect = CGRect::new(CGPoint::new(0.0, 0.0), size);
        let bitmap: *mut AnyObject = msg_send![bitmap, initWithFocusedViewRect: rect];

        // Unlock focus
        let _: () = msg_send![image, unlockFocus];

        if bitmap.is_null() {
            return None;
        }

        // Convert to PNG data
        let png_data: *mut AnyObject = msg_send![
            bitmap,
            representationUsingType: NS_PNG_FILE_TYPE
            properties: std::ptr::null::<c_void>()
        ];

        if png_data.is_null() {
            return None;
        }

        // Get the bytes from NSData
        let length: usize = msg_send![png_data, length];
        let bytes: *const u8 = msg_send![png_data, bytes];

        if bytes.is_null() || length == 0 {
            return None;
        }

        // Copy the data
        let slice = std::slice::from_raw_parts(bytes, length);
        Some(slice.to_vec())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: These tests require AppKit to be linked, which doesn't happen in
    // pure CLI test runs. They work when run within the Tauri app context.
    // Run with: cargo test --manifest-path crates/platform-macos/Cargo.toml file_icon -- --ignored

    #[test]
    #[ignore = "requires AppKit to be linked (run in GUI context)"]
    fn icon_of_existing_file() {
        // Test with a file that definitely exists
        let result = icon_of_path("/System/Library/CoreServices/Finder.app");
        assert!(result.is_some(), "should get icon for Finder.app");

        let data_uri = result.unwrap();
        assert!(
            data_uri.starts_with("data:image/png;base64,"),
            "should be a PNG data URI"
        );
    }

    #[test]
    #[ignore = "requires AppKit to be linked (run in GUI context)"]
    fn icon_of_directory() {
        let result = icon_of_path("/Applications");
        assert!(result.is_some(), "should get icon for /Applications");
    }

    #[test]
    #[ignore = "requires AppKit to be linked (run in GUI context)"]
    fn icon_of_nonexistent_file() {
        // NSWorkspace still returns a generic icon for nonexistent files
        let result = icon_of_path("/nonexistent/path/file.xyz");
        // This may or may not return an icon depending on macOS behavior
        // Just verify it doesn't crash
        let _ = result;
    }

    #[test]
    #[ignore = "requires AppKit to be linked (run in GUI context)"]
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
