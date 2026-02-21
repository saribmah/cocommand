//! Editor extension tests.

use std::fs;

use super::extension::EditorExtension;
use super::ops;

// ── Extension ─────────────────────────────────────────────────────────

#[test]
fn editor_extension_has_expected_tools() {
    let ext = EditorExtension::new();
    let tools = crate::extension::Extension::tools(&ext);
    assert_eq!(tools.len(), 3);
    assert_eq!(tools[0].id, "read_file");
    assert_eq!(tools[1].id, "write_file");
    assert_eq!(tools[2].id, "edit_file");
}

// ── read_file ─────────────────────────────────────────────────────────

#[test]
fn read_file_basic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("test.txt");
    fs::write(
        &file,
        "line one\nline two\nline three\nline four\nline five\n",
    )
    .expect("write");

    let result = ops::read_file(&file, 1, 2000).expect("read_file");
    assert_eq!(result["lines"].as_u64().unwrap(), 5);
    let content = result["content"].as_str().unwrap();
    assert!(content.contains("1\tline one"));
    assert!(content.contains("5\tline five"));
}

#[test]
fn read_file_with_offset_and_limit() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("test.txt");
    let lines: Vec<String> = (1..=10).map(|i| format!("line {i}")).collect();
    fs::write(&file, lines.join("\n")).expect("write");

    let result = ops::read_file(&file, 3, 3).expect("read_file");
    let content = result["content"].as_str().unwrap();
    assert!(content.contains("3\tline 3"));
    assert!(content.contains("4\tline 4"));
    assert!(content.contains("5\tline 5"));
    assert!(!content.contains("2\t"));
    assert!(!content.contains("6\t"));
    assert!(result["truncated"].as_bool().unwrap());
}

#[test]
fn read_file_offset_beyond_end() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("test.txt");
    fs::write(&file, "one\ntwo\nthree\n").expect("write");

    let result = ops::read_file(&file, 100, 10);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("beyond end"));
}

#[test]
fn read_file_binary_detection() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("binary.bin");
    // Write >30% non-printable bytes
    let mut data = vec![0u8; 100];
    for (i, byte) in data.iter_mut().enumerate() {
        if i < 40 {
            *byte = 0x00; // non-printable
        } else {
            *byte = b'A'; // printable
        }
    }
    fs::write(&file, &data).expect("write");

    let result = ops::read_file(&file, 1, 2000);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("binary"));
}

#[test]
fn read_file_empty_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("empty.txt");
    fs::write(&file, "").expect("write");

    let result = ops::read_file(&file, 1, 2000).expect("read_file");
    assert_eq!(result["lines"].as_u64().unwrap(), 0);
    assert_eq!(result["content"].as_str().unwrap(), "");
}

#[test]
fn read_file_nonexistent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("nope.txt");
    let result = ops::read_file(&file, 1, 2000);
    assert!(result.is_err());
}

#[test]
fn read_file_directory_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let result = ops::read_file(dir.path(), 1, 2000);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("directory") || err.contains("ls"));
}

#[test]
fn read_file_long_lines_truncated() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("long.txt");
    let long_line = "x".repeat(3000);
    fs::write(&file, &long_line).expect("write");

    let result = ops::read_file(&file, 1, 2000).expect("read_file");
    let content = result["content"].as_str().unwrap();
    // The line content (after the line number + tab) should be truncated
    // Line format is "1\t" + content + "\n"
    let line = content.lines().next().unwrap();
    let tab_idx = line.find('\t').unwrap();
    let line_content = &line[tab_idx + 1..];
    assert!(line_content.len() <= 2000);
}

// ── write_file ────────────────────────────────────────────────────────

#[test]
fn write_file_creates_new() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("new.txt");

    let result = ops::write_file(&file, "hello world").expect("write_file");
    assert!(result["created"].as_bool().unwrap());
    assert_eq!(result["bytes_written"].as_u64().unwrap(), 11);
    assert_eq!(fs::read_to_string(&file).unwrap(), "hello world");
}

#[test]
fn write_file_overwrites_existing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("existing.txt");
    fs::write(&file, "old content").expect("write");

    let result = ops::write_file(&file, "new content").expect("write_file");
    assert!(!result["created"].as_bool().unwrap());
    assert_eq!(fs::read_to_string(&file).unwrap(), "new content");
}

#[test]
fn write_file_creates_parent_dirs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("a/b/c/deep.txt");

    let result = ops::write_file(&file, "nested").expect("write_file");
    assert!(result["created"].as_bool().unwrap());
    assert_eq!(fs::read_to_string(&file).unwrap(), "nested");
}

// ── edit_file ─────────────────────────────────────────────────────────

#[test]
fn edit_file_single_replacement() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("edit.txt");
    fs::write(&file, "hello world, hello rust").expect("write");

    // "hello world" appears once as an exact substring
    let result = ops::edit_file(&file, "hello world", "goodbye world", false).expect("edit_file");
    assert_eq!(result["replacements"].as_u64().unwrap(), 1);
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "goodbye world, hello rust"
    );
}

#[test]
fn edit_file_replace_all() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("edit.txt");
    fs::write(&file, "aaa bbb aaa ccc aaa").expect("write");

    let result = ops::edit_file(&file, "aaa", "zzz", true).expect("edit_file");
    assert_eq!(result["replacements"].as_u64().unwrap(), 3);
    assert_eq!(fs::read_to_string(&file).unwrap(), "zzz bbb zzz ccc zzz");
}

#[test]
fn edit_file_multiple_matches_no_replace_all() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("edit.txt");
    fs::write(&file, "foo bar foo").expect("write");

    let result = ops::edit_file(&file, "foo", "baz", false);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("matches"));
}

#[test]
fn edit_file_not_found_in_content() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("edit.txt");
    fs::write(&file, "hello world").expect("write");

    let result = ops::edit_file(&file, "not here", "replacement", false);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not found"));
}

#[test]
fn edit_file_old_equals_new() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("edit.txt");
    fs::write(&file, "hello").expect("write");

    let result = ops::edit_file(&file, "hello", "hello", false);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("different"));
}

#[test]
fn edit_file_trimmed_fallback() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("edit.txt");
    fs::write(
        &file,
        "    fn main() {\n        println!(\"hello\");\n    }\n",
    )
    .expect("write");

    // old_string has different indentation
    let result = ops::edit_file(
        &file,
        "fn main() {\n    println!(\"hello\");\n}",
        "fn main() {\n    println!(\"goodbye\");\n}",
        false,
    )
    .expect("edit_file trimmed");
    assert_eq!(result["replacements"].as_u64().unwrap(), 1);
    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("goodbye"));
}

#[test]
fn edit_file_additions_deletions() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("edit.txt");
    fs::write(&file, "line1\nline2\n").expect("write");

    // Replace 2 lines with 4 lines
    let result =
        ops::edit_file(&file, "line1\nline2", "line1\nline2\nline3\nline4", false).expect("edit");
    assert_eq!(result["replacements"].as_u64().unwrap(), 1);
    assert_eq!(result["additions"].as_u64().unwrap(), 2);
    assert_eq!(result["deletions"].as_u64().unwrap(), 0);
}

#[test]
fn edit_file_delete_text() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("edit.txt");
    fs::write(&file, "keep this\ndelete this\nkeep this too\n").expect("write");

    let result = ops::edit_file(&file, "delete this\n", "", false).expect("edit");
    assert_eq!(result["replacements"].as_u64().unwrap(), 1);
    assert!(result["deletions"].as_u64().unwrap() > 0);
    let content = fs::read_to_string(&file).unwrap();
    assert!(!content.contains("delete this"));
}
