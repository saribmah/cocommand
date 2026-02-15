use std::path::PathBuf;

use tempfile::tempdir;

use crate::error::CoreError;

use super::ops::{
    build_search_payload, create_note, ensure_notes_dir, note_id_from_path, note_path_from_id,
    notes_root, sanitize_note_slug,
};

#[test]
fn note_path_from_id_rejects_relative_segments() {
    let root = PathBuf::from("/tmp/notes");
    let error = note_path_from_id(&root, "../secret").expect_err("invalid id should fail");
    match error {
        CoreError::InvalidInput(message) => {
            assert!(
                message.contains("relative path segments"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected invalid input error, got {other:?}"),
    }
}

#[test]
fn note_id_roundtrips_with_nested_paths() {
    let dir = tempdir().expect("tempdir");
    let root = notes_root(dir.path());
    ensure_notes_dir(&root).expect("create notes root");
    let nested = root.join("daily/todo.md");
    std::fs::create_dir_all(nested.parent().expect("parent")).expect("create nested dir");
    std::fs::write(&nested, "# Todo").expect("write note");

    let id = note_id_from_path(&root, &nested).expect("id from path");
    assert_eq!(id, "daily/todo");

    let resolved = note_path_from_id(&root, &id).expect("path from id");
    assert_eq!(resolved, nested);
}

#[test]
fn sanitize_note_slug_falls_back_to_untitled() {
    assert_eq!(sanitize_note_slug("    "), "untitled");
    assert_eq!(sanitize_note_slug("###"), "untitled");
}

#[test]
fn create_note_generates_unique_ids() {
    let dir = tempdir().expect("tempdir");
    let root = notes_root(dir.path());

    let first = create_note(root.clone(), Some("Daily Plan".to_string()), None, None)
        .expect("create first note");
    let second = create_note(root.clone(), Some("Daily Plan".to_string()), None, None)
        .expect("create second note");

    assert_eq!(first.id, "daily-plan");
    assert_eq!(second.id, "daily-plan-1");
}

#[test]
fn search_payload_filters_non_markdown_files() {
    let dir = tempdir().expect("tempdir");
    let root = notes_root(dir.path());
    ensure_notes_dir(&root).expect("create notes dir");

    let note_path = root.join("project.md");
    let png_path = root.join("diagram.png");
    std::fs::write(&note_path, "# Project\n\nStatus").expect("write markdown note");
    std::fs::write(&png_path, "binary").expect("write png placeholder");

    let search_result = filesystem::SearchResult {
        query: "project".to_string(),
        root: root.to_string_lossy().to_string(),
        entries: vec![
            filesystem::FileEntry {
                path: note_path.to_string_lossy().to_string(),
                name: "project.md".to_string(),
                file_type: filesystem::FileType::File,
                size: None,
                modified_at: None,
            },
            filesystem::FileEntry {
                path: png_path.to_string_lossy().to_string(),
                name: "diagram.png".to_string(),
                file_type: filesystem::FileType::File,
                size: None,
                modified_at: None,
            },
        ],
        count: 2,
        truncated: false,
        scanned: 2,
        errors: 0,
        index_state: "ready".to_string(),
        index_scanned_files: 2,
        index_scanned_dirs: 1,
        index_started_at: None,
        index_last_update_at: None,
        index_finished_at: None,
        highlight_terms: vec!["project".to_string()],
    };

    let payload = build_search_payload(&root, "project".to_string(), Some(search_result));
    assert_eq!(payload.notes.len(), 1);
    assert_eq!(payload.notes[0].id, "project");
}
