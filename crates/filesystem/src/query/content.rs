//! Content search using Rabin-Karp algorithm from memchr.
//!
//! This module provides file content searching with:
//! - Single-byte optimization for single character needles
//! - Rabin-Karp for multi-byte needles
//! - Overlap handling for matches spanning buffer boundaries
//! - Case-insensitive matching support
//! - Cancellation support via CancellationToken

use std::fs::File;
use std::io::Read;
use std::path::Path;

use memchr::arch::all::rabinkarp;

use crate::cancel::CancellationToken;

/// Buffer size for file content reading (64KB).
pub const CONTENT_BUFFER_BYTES: usize = 64 * 1024;

/// Searches file contents for the given needle.
///
/// # Arguments
/// * `path` - Path to the file to search
/// * `needle` - The bytes to search for (must be lowercased if case_insensitive)
/// * `case_insensitive` - Whether to perform case-insensitive matching
/// * `token` - Cancellation token for early termination
///
/// # Returns
/// * `Some(true)` - File contains the needle
/// * `Some(false)` - File does not contain the needle (or read error)
/// * `None` - Search was cancelled
pub fn file_content_matches(
    path: &Path,
    needle: &[u8],
    case_insensitive: bool,
    token: CancellationToken,
) -> Option<bool> {
    token.is_cancelled()?;

    let Ok(mut file) = File::open(path) else {
        return Some(false);
    };

    if needle.is_empty() {
        return Some(false);
    }

    // Single-byte optimization: use simple byte search
    if needle.len() == 1 {
        return search_single_byte(&mut file, needle[0], case_insensitive, token);
    }

    // Multi-byte: use Rabin-Karp algorithm
    search_multi_byte(&mut file, needle, case_insensitive, token)
}

/// Searches for a single byte in the file.
fn search_single_byte(
    file: &mut File,
    needle: u8,
    case_insensitive: bool,
    token: CancellationToken,
) -> Option<bool> {
    let mut buffer = vec![0u8; CONTENT_BUFFER_BYTES];

    if case_insensitive {
        let lowercase_target = needle.to_ascii_lowercase();
        let uppercase_target = needle.to_ascii_uppercase();

        loop {
            token.is_cancelled()?;

            let read = match file.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => count,
                Err(_) => return Some(false),
            };

            if buffer[..read]
                .iter()
                .any(|&c| c == lowercase_target || c == uppercase_target)
            {
                return Some(true);
            }
        }
    } else {
        loop {
            token.is_cancelled()?;

            let read = match file.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => count,
                Err(_) => return Some(false),
            };

            if buffer[..read].contains(&needle) {
                return Some(true);
            }
        }
    }

    Some(false)
}

/// Searches for a multi-byte needle using Rabin-Karp algorithm.
///
/// Uses overlap handling to detect matches that span buffer boundaries.
fn search_multi_byte(
    file: &mut File,
    needle: &[u8],
    case_insensitive: bool,
    token: CancellationToken,
) -> Option<bool> {
    // Ensure needle is lowercased if case_insensitive is set
    debug_assert!(
        !case_insensitive || needle == needle.to_ascii_lowercase(),
        "needle must be lowercased when case_insensitive is true"
    );

    // Overlap size: keep needle.len() - 1 bytes from previous chunk
    // to handle matches spanning buffer boundaries
    let overlap = needle.len().saturating_sub(1);
    let finder = rabinkarp::Finder::new(needle);

    // Buffer includes space for overlap from previous iteration
    let mut buffer = vec![0u8; CONTENT_BUFFER_BYTES + overlap];
    let mut carry_len = 0usize;

    loop {
        token.is_cancelled()?;

        // Read into buffer after the carry bytes
        let Ok(read) = file.read(&mut buffer[carry_len..]) else {
            return Some(false);
        };

        if read == 0 {
            break;
        }

        let chunk_len = carry_len + read;
        let chunk = &mut buffer[..chunk_len];

        // Convert newly read bytes to lowercase if case-insensitive
        if case_insensitive {
            chunk[carry_len..].make_ascii_lowercase();
        }

        // Search for needle in current chunk
        if finder.find(chunk, needle).is_some() {
            return Some(true);
        }

        // Preserve overlap bytes for next iteration
        let keep = overlap.min(chunk.len());
        if keep > 0 {
            let start = chunk.len().saturating_sub(keep);
            buffer.copy_within(start..chunk_len, 0);
        }
        carry_len = keep;
    }

    Some(false)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_temp_file(content: &[u8]) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn empty_needle_returns_false() {
        let file = create_temp_file(b"hello world");
        let result = file_content_matches(file.path(), b"", false, CancellationToken::noop());
        assert_eq!(result, Some(false));
    }

    #[test]
    fn single_byte_found_case_sensitive() {
        let file = create_temp_file(b"hello world");
        let result = file_content_matches(file.path(), b"o", false, CancellationToken::noop());
        assert_eq!(result, Some(true));
    }

    #[test]
    fn single_byte_not_found_case_sensitive() {
        let file = create_temp_file(b"hello world");
        let result = file_content_matches(file.path(), b"X", false, CancellationToken::noop());
        assert_eq!(result, Some(false));
    }

    #[test]
    fn single_byte_case_insensitive() {
        let file = create_temp_file(b"Hello World");
        // Searching for lowercase 'h' should match uppercase 'H'
        let result = file_content_matches(file.path(), b"h", true, CancellationToken::noop());
        assert_eq!(result, Some(true));

        // Searching for uppercase 'W' should match
        let result = file_content_matches(file.path(), b"w", true, CancellationToken::noop());
        assert_eq!(result, Some(true));
    }

    #[test]
    fn multi_byte_found_case_sensitive() {
        let file = create_temp_file(b"hello world");
        let result = file_content_matches(file.path(), b"world", false, CancellationToken::noop());
        assert_eq!(result, Some(true));
    }

    #[test]
    fn multi_byte_not_found_case_sensitive() {
        let file = create_temp_file(b"hello world");
        let result = file_content_matches(file.path(), b"WORLD", false, CancellationToken::noop());
        assert_eq!(result, Some(false));
    }

    #[test]
    fn multi_byte_case_insensitive() {
        let file = create_temp_file(b"Hello World");
        // Needle must be lowercased for case-insensitive search
        let result = file_content_matches(file.path(), b"world", true, CancellationToken::noop());
        assert_eq!(result, Some(true));

        let result = file_content_matches(file.path(), b"hello", true, CancellationToken::noop());
        assert_eq!(result, Some(true));
    }

    #[test]
    fn nonexistent_file_returns_false() {
        let result = file_content_matches(
            Path::new("/nonexistent/file/path"),
            b"test",
            false,
            CancellationToken::noop(),
        );
        assert_eq!(result, Some(false));
    }

    #[test]
    fn match_at_buffer_boundary() {
        // Create content that ensures the needle spans a buffer boundary
        // We need the needle to start near the end of one buffer and end in the next
        let mut content = vec![b'A'; CONTENT_BUFFER_BYTES - 3];
        content.extend_from_slice(b"NEEDLE"); // "NEEDLE" spans boundary
        content.extend_from_slice(&vec![b'B'; 100]);

        let file = create_temp_file(&content);
        let result = file_content_matches(file.path(), b"NEEDLE", false, CancellationToken::noop());
        assert_eq!(result, Some(true));
    }

    #[test]
    fn match_at_exact_buffer_boundary() {
        // Place needle exactly at buffer boundary (last 3 bytes + first 3 bytes)
        let mut content = vec![b'A'; CONTENT_BUFFER_BYTES - 3];
        content.extend_from_slice(b"XYZ123"); // Spans exactly
        content.extend_from_slice(&vec![b'B'; 100]);

        let file = create_temp_file(&content);
        let result = file_content_matches(file.path(), b"XYZ123", false, CancellationToken::noop());
        assert_eq!(result, Some(true));
    }

    #[test]
    fn binary_content_with_null_bytes() {
        let content = b"hello\x00world\x00test";
        let file = create_temp_file(content);

        let result = file_content_matches(file.path(), b"world", false, CancellationToken::noop());
        assert_eq!(result, Some(true));

        let result = file_content_matches(file.path(), b"\x00", false, CancellationToken::noop());
        assert_eq!(result, Some(true));
    }

    #[test]
    fn utf8_multibyte_characters() {
        let content = "Hello 世界 🌍".as_bytes();
        let file = create_temp_file(content);

        let result = file_content_matches(
            file.path(),
            "世界".as_bytes(),
            false,
            CancellationToken::noop(),
        );
        assert_eq!(result, Some(true));

        let result = file_content_matches(
            file.path(),
            "🌍".as_bytes(),
            false,
            CancellationToken::noop(),
        );
        assert_eq!(result, Some(true));
    }

    #[test]
    fn needle_longer_than_buffer() {
        // Create a needle longer than the buffer size
        let long_needle = vec![b'X'; CONTENT_BUFFER_BYTES + 100];
        let mut content = vec![b'A'; 1000];
        content.extend_from_slice(&long_needle);
        content.extend_from_slice(&vec![b'B'; 1000]);

        let file = create_temp_file(&content);
        let result =
            file_content_matches(file.path(), &long_needle, false, CancellationToken::noop());
        assert_eq!(result, Some(true));
    }

    #[test]
    fn large_file_search() {
        // Create a file larger than one buffer
        let mut content = vec![b'A'; CONTENT_BUFFER_BYTES * 3];
        content.extend_from_slice(b"FINDME");
        content.extend_from_slice(&vec![b'B'; 1000]);

        let file = create_temp_file(&content);
        let result = file_content_matches(file.path(), b"FINDME", false, CancellationToken::noop());
        assert_eq!(result, Some(true));
    }

    #[test]
    fn needle_at_start_of_file() {
        let file = create_temp_file(b"NEEDLE followed by other content");
        let result = file_content_matches(file.path(), b"NEEDLE", false, CancellationToken::noop());
        assert_eq!(result, Some(true));
    }

    #[test]
    fn needle_at_end_of_file() {
        let file = create_temp_file(b"content before NEEDLE");
        let result = file_content_matches(file.path(), b"NEEDLE", false, CancellationToken::noop());
        assert_eq!(result, Some(true));
    }

    #[test]
    fn empty_file() {
        let file = create_temp_file(b"");
        let result = file_content_matches(file.path(), b"test", false, CancellationToken::noop());
        assert_eq!(result, Some(false));
    }
}
