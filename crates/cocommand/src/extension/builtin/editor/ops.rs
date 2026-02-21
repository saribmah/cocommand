//! Editor operation implementations.

use std::path::Path;

use crate::error::{CoreError, CoreResult};

const MAX_LINE_CHARS: usize = 2_000;

/// Read a file with line numbers.
///
/// Returns numbered lines formatted as `"  42\tcontent"` with right-aligned
/// line numbers. Detects binary files and rejects them.
pub fn read_file(path: &Path, offset: u64, limit: u64) -> CoreResult<serde_json::Value> {
    if path.is_dir() {
        return Err(CoreError::InvalidInput(format!(
            "{} is a directory, not a file. Use the ls tool to list directory contents.",
            path.display()
        )));
    }

    let bytes = std::fs::read(path).map_err(|e| {
        CoreError::InvalidInput(format!("failed to read {}: {e}", path.display()))
    })?;

    // Binary detection: check first 4KB for >30% non-printable chars
    let check_len = bytes.len().min(4096);
    if check_len > 0 {
        let non_printable = bytes[..check_len]
            .iter()
            .filter(|&&b| {
                // Non-printable: not tab, newline, carriage return, and outside printable ASCII
                b != b'\t' && b != b'\n' && b != b'\r' && (b < 0x20 || b > 0x7E)
            })
            .count();
        let ratio = non_printable as f64 / check_len as f64;
        if ratio > 0.3 {
            return Err(CoreError::InvalidInput(format!(
                "{} appears to be a binary file and cannot be read as text.",
                path.display()
            )));
        }
    }

    let content_str = String::from_utf8_lossy(&bytes);
    let all_lines: Vec<&str> = content_str.lines().collect();
    let total_lines = all_lines.len();

    // offset is 1-indexed
    let offset_usize = offset as usize;
    if offset_usize > 1 && offset_usize > total_lines {
        return Err(CoreError::InvalidInput(format!(
            "offset {} is beyond end of file ({} lines)",
            offset, total_lines
        )));
    }

    let start = if offset_usize >= 1 {
        offset_usize - 1
    } else {
        0
    };
    let end = (start + limit as usize).min(total_lines);
    let selected = &all_lines[start..end];

    // Determine width for right-aligned line numbers
    let max_lineno = if selected.is_empty() {
        1
    } else {
        end
    };
    let width = max_lineno.to_string().len();

    let mut output = String::new();
    for (i, line) in selected.iter().enumerate() {
        let lineno = start + i + 1;
        let mut line_content = line.to_string();
        if line_content.len() > MAX_LINE_CHARS {
            line_content.truncate(line_content.floor_char_boundary(MAX_LINE_CHARS));
        }
        output.push_str(&format!("{lineno:>width$}\t{line_content}\n"));
    }

    let truncated = end < total_lines;

    Ok(serde_json::json!({
        "path": path.to_string_lossy(),
        "content": output,
        "lines": total_lines,
        "offset": offset,
        "limit": limit,
        "truncated": truncated,
    }))
}

/// Write content to a file, creating parent directories if needed.
pub fn write_file(path: &Path, content: &str) -> CoreResult<serde_json::Value> {
    let created = !path.exists();

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CoreError::Internal(format!(
                    "failed to create parent directories for {}: {e}",
                    path.display()
                ))
            })?;
        }
    }

    let bytes = content.as_bytes();
    std::fs::write(path, bytes).map_err(|e| {
        CoreError::Internal(format!("failed to write {}: {e}", path.display()))
    })?;

    Ok(serde_json::json!({
        "path": path.to_string_lossy(),
        "bytes_written": bytes.len(),
        "created": created,
    }))
}

/// Find and replace text in a file.
///
/// If exact match fails, tries a whitespace-trimmed line-by-line fallback.
pub fn edit_file(
    path: &Path,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> CoreResult<serde_json::Value> {
    if old_string == new_string {
        return Err(CoreError::InvalidInput(
            "old_string and new_string must be different".to_string(),
        ));
    }

    let content = std::fs::read_to_string(path).map_err(|e| {
        CoreError::InvalidInput(format!("failed to read {}: {e}", path.display()))
    })?;

    let exact_count = content.matches(old_string).count();

    let (new_content, replacements) = if exact_count > 0 {
        if exact_count > 1 && !replace_all {
            return Err(CoreError::InvalidInput(format!(
                "found {} matches for old_string — provide more context to make it unique or set replace_all to true",
                exact_count
            )));
        }
        let replaced = if replace_all {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };
        let count = if replace_all { exact_count } else { 1 };
        (replaced, count)
    } else {
        // Try trimmed fallback
        match try_trimmed_replace(&content, old_string, new_string, replace_all)? {
            Some((replaced, count)) => (replaced, count),
            None => {
                return Err(CoreError::InvalidInput(
                    "old_string not found in file".to_string(),
                ));
            }
        }
    };

    std::fs::write(path, &new_content).map_err(|e| {
        CoreError::Internal(format!("failed to write {}: {e}", path.display()))
    })?;

    let old_line_count = old_string.lines().count() as i64;
    let new_line_count = new_string.lines().count() as i64;
    // If old_string is empty, old_line_count is 0 from lines().count()
    let diff = (new_line_count - old_line_count) * replacements as i64;
    let additions = if diff > 0 { diff as u64 } else { 0 };
    let deletions = if diff < 0 { (-diff) as u64 } else { 0 };

    Ok(serde_json::json!({
        "path": path.to_string_lossy(),
        "replacements": replacements,
        "additions": additions,
        "deletions": deletions,
    }))
}

/// Attempt a whitespace-trimmed line-by-line match as a fallback.
///
/// Splits `old_string` into lines, trims each, and searches for a contiguous
/// block in the file content where trimmed lines match.
fn try_trimmed_replace(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> CoreResult<Option<(String, usize)>> {
    let old_lines: Vec<&str> = old_string.lines().collect();
    if old_lines.is_empty() {
        return Ok(None);
    }

    let content_lines: Vec<&str> = content.lines().collect();
    let old_trimmed: Vec<&str> = old_lines.iter().map(|l| l.trim()).collect();

    // Find all contiguous blocks where trimmed lines match
    let mut match_starts: Vec<usize> = Vec::new();
    for i in 0..content_lines.len() {
        if i + old_lines.len() > content_lines.len() {
            break;
        }
        let matches = (0..old_lines.len())
            .all(|j| content_lines[i + j].trim() == old_trimmed[j]);
        if matches {
            match_starts.push(i);
        }
    }

    if match_starts.is_empty() {
        return Ok(None);
    }

    if match_starts.len() > 1 && !replace_all {
        return Err(CoreError::InvalidInput(format!(
            "found {} matches for old_string (trimmed) — provide more context to make it unique or set replace_all to true",
            match_starts.len()
        )));
    }

    // Replace from the end so indices stay valid
    let mut result_lines: Vec<&str> = content_lines;
    let new_string_lines: Vec<&str> = new_string.lines().collect();
    let count = if replace_all {
        match_starts.len()
    } else {
        1
    };
    let starts_to_replace = if replace_all {
        match_starts.clone()
    } else {
        vec![match_starts[0]]
    };

    // Replace from last to first to keep indices valid
    for &start in starts_to_replace.iter().rev() {
        let end = start + old_lines.len();
        result_lines.splice(start..end, new_string_lines.iter().copied());
    }

    let mut replaced = result_lines.join("\n");
    // Preserve trailing newline if original had one
    if content.ends_with('\n') {
        replaced.push('\n');
    }

    Ok(Some((replaced, count)))
}
