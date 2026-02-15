use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::error::{CoreError, CoreResult};

use super::types::{ListNotesPayload, NotePayload, NoteSummaryPayload, SearchNotesPayload};

const DEFAULT_NOTE_TITLE: &str = "Untitled";
const DEFAULT_NOTE_SLUG: &str = "untitled";
const MAX_TITLE_CHARS: usize = 120;
const MAX_PREVIEW_CHARS: usize = 200;

pub fn notes_root(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("notes")
}

pub fn notes_index_cache_dir(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("storage/notes-indexes")
}

pub fn ensure_notes_dir(notes_root: &Path) -> CoreResult<()> {
    fs::create_dir_all(notes_root).map_err(|error| {
        CoreError::Internal(format!(
            "failed to create notes directory {}: {error}",
            notes_root.display()
        ))
    })?;
    Ok(())
}

pub fn note_id_from_path(notes_root: &Path, file_path: &Path) -> Option<String> {
    let rel = file_path.strip_prefix(notes_root).ok()?;
    if file_path.extension()?.to_str()? != "md" {
        return None;
    }

    let mut parts = Vec::new();
    for component in rel.components() {
        let std::path::Component::Normal(raw) = component else {
            return None;
        };
        let value = raw.to_str()?;
        if value.starts_with('.') || value.is_empty() {
            return None;
        }
        parts.push(value.to_string());
    }

    let last = parts.pop()?;
    let stem = last.strip_suffix(".md")?;
    if stem.is_empty() {
        return None;
    }
    parts.push(stem.to_string());
    Some(parts.join("/"))
}

pub fn note_path_from_id(notes_root: &Path, id: &str) -> CoreResult<PathBuf> {
    let segments = normalize_id_segments(id, "id")?;
    let mut relative = PathBuf::new();
    for segment in segments {
        relative.push(segment);
    }
    let joined = notes_root.join(relative);
    let mut os = joined.into_os_string();
    os.push(".md");
    let file_path = PathBuf::from(os);
    if !file_path.starts_with(notes_root) {
        return Err(CoreError::InvalidInput(
            "invalid note id: path escapes notes root".to_string(),
        ));
    }
    Ok(file_path)
}

pub fn list_notes(notes_root: PathBuf, limit: usize) -> CoreResult<ListNotesPayload> {
    ensure_notes_dir(&notes_root)?;
    let mut stack = vec![notes_root.clone()];
    let mut notes = Vec::new();
    let mut errors = 0usize;

    while let Some(current_dir) = stack.pop() {
        let directory_iter = match fs::read_dir(&current_dir) {
            Ok(iter) => iter,
            Err(_) => {
                errors += 1;
                continue;
            }
        };

        for entry_result in directory_iter {
            let entry = match entry_result {
                Ok(entry) => entry,
                Err(_) => {
                    errors += 1;
                    continue;
                }
            };
            let path = entry.path();
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => {
                    errors += 1;
                    continue;
                }
            };
            if metadata.is_dir() {
                if !should_skip_directory(&path) {
                    stack.push(path);
                }
                continue;
            }
            if !metadata.is_file() {
                continue;
            }
            if let Some(note) = summarize_note_file(&notes_root, &path, Some(&metadata)) {
                notes.push(note);
            }
        }
    }

    notes.sort_by(|left, right| {
        right
            .modified_at
            .cmp(&left.modified_at)
            .then_with(|| left.id.cmp(&right.id))
    });

    let truncated = notes.len() > limit;
    if truncated {
        notes.truncate(limit);
    }

    Ok(ListNotesPayload {
        root: notes_root.to_string_lossy().to_string(),
        count: notes.len(),
        notes,
        truncated,
        errors,
    })
}

pub fn read_note(notes_root: PathBuf, id: String) -> CoreResult<NotePayload> {
    ensure_notes_dir(&notes_root)?;
    let path = note_path_from_id(&notes_root, &id)?;
    if !path.exists() {
        return Err(CoreError::InvalidInput(format!(
            "note not found: {id} ({})",
            path.display()
        )));
    }
    let content = fs::read_to_string(&path).map_err(|error| {
        CoreError::Internal(format!("failed to read note {}: {error}", path.display()))
    })?;
    let metadata = fs::metadata(&path).map_err(|error| {
        CoreError::Internal(format!(
            "failed to read note metadata {}: {error}",
            path.display()
        ))
    })?;

    Ok(NotePayload {
        id,
        title: extract_title(&content),
        preview: generate_preview(&content),
        content,
        path: path.to_string_lossy().to_string(),
        modified_at: modified_secs(&metadata),
        size: Some(metadata.len()),
    })
}

pub fn create_note(
    notes_root: PathBuf,
    title: Option<String>,
    content: Option<String>,
    folder: Option<String>,
) -> CoreResult<NotePayload> {
    ensure_notes_dir(&notes_root)?;
    let raw_content = content.unwrap_or_default();
    let title = normalize_create_title(title.as_deref(), &raw_content);
    let base_slug = sanitize_note_slug(&title);
    let folder_prefix = normalize_optional_folder(folder.as_deref())?;
    let note_id = next_available_note_id(&notes_root, &folder_prefix, &base_slug)?;
    let note_path = note_path_from_id(&notes_root, &note_id)?;
    if let Some(parent) = note_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            CoreError::Internal(format!(
                "failed to create note parent directory {}: {error}",
                parent.display()
            ))
        })?;
    }

    let final_content = if raw_content.trim().is_empty() {
        format!("# {title}\n\n")
    } else {
        raw_content
    };
    fs::write(&note_path, final_content.as_bytes()).map_err(|error| {
        CoreError::Internal(format!(
            "failed to write note {}: {error}",
            note_path.display()
        ))
    })?;

    read_note(notes_root, note_id)
}

pub fn update_note(notes_root: PathBuf, id: String, content: String) -> CoreResult<NotePayload> {
    ensure_notes_dir(&notes_root)?;
    let note_path = note_path_from_id(&notes_root, &id)?;
    if !note_path.exists() {
        return Err(CoreError::InvalidInput(format!(
            "note not found: {id} ({})",
            note_path.display()
        )));
    }
    fs::write(&note_path, content.as_bytes()).map_err(|error| {
        CoreError::Internal(format!(
            "failed to update note {}: {error}",
            note_path.display()
        ))
    })?;
    read_note(notes_root, id)
}

pub fn delete_note(notes_root: PathBuf, id: String) -> CoreResult<bool> {
    ensure_notes_dir(&notes_root)?;
    let note_path = note_path_from_id(&notes_root, &id)?;
    if !note_path.exists() {
        return Ok(false);
    }
    fs::remove_file(&note_path).map_err(|error| {
        CoreError::Internal(format!(
            "failed to delete note {}: {error}",
            note_path.display()
        ))
    })?;
    cleanup_empty_note_dirs(&notes_root, note_path.parent());
    Ok(true)
}

pub fn build_search_payload(
    notes_root: &Path,
    query: String,
    result: Option<filesystem::SearchResult>,
) -> SearchNotesPayload {
    let Some(result) = result else {
        return SearchNotesPayload {
            query,
            root: notes_root.to_string_lossy().to_string(),
            notes: Vec::new(),
            count: 0,
            truncated: false,
            scanned: 0,
            errors: 0,
            index_state: "cancelled".to_string(),
            index_scanned_files: 0,
            index_scanned_dirs: 0,
            index_started_at: None,
            index_last_update_at: None,
            index_finished_at: None,
            highlight_terms: Vec::new(),
        };
    };

    let notes = result
        .entries
        .into_iter()
        .filter_map(|entry| summarize_search_entry(notes_root, entry))
        .collect::<Vec<_>>();
    SearchNotesPayload {
        query: result.query,
        root: result.root,
        count: notes.len(),
        notes,
        truncated: result.truncated,
        scanned: result.scanned,
        errors: result.errors,
        index_state: result.index_state,
        index_scanned_files: result.index_scanned_files,
        index_scanned_dirs: result.index_scanned_dirs,
        index_started_at: result.index_started_at,
        index_last_update_at: result.index_last_update_at,
        index_finished_at: result.index_finished_at,
        highlight_terms: result.highlight_terms,
    }
}

pub fn sanitize_note_slug(input: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for ch in input.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_dash = false;
            continue;
        }
        if !slug.is_empty() && !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        DEFAULT_NOTE_SLUG.to_string()
    } else {
        slug
    }
}

fn summarize_search_entry(
    notes_root: &Path,
    entry: filesystem::FileEntry,
) -> Option<NoteSummaryPayload> {
    let path = PathBuf::from(&entry.path);
    let id = note_id_from_path(notes_root, &path)?;
    let content = fs::read_to_string(&path).unwrap_or_default();
    let title = if content.is_empty() {
        fallback_title_from_id(&id)
    } else {
        extract_title(&content)
    };
    let preview = if content.is_empty() {
        String::new()
    } else {
        generate_preview(&content)
    };
    Some(NoteSummaryPayload {
        id,
        title,
        preview,
        path: entry.path,
        modified_at: entry.modified_at,
        size: entry.size,
    })
}

fn summarize_note_file(
    notes_root: &Path,
    path: &Path,
    metadata: Option<&fs::Metadata>,
) -> Option<NoteSummaryPayload> {
    let id = note_id_from_path(notes_root, path)?;
    let content = fs::read_to_string(path).unwrap_or_default();
    let title = if content.is_empty() {
        fallback_title_from_id(&id)
    } else {
        extract_title(&content)
    };
    let preview = if content.is_empty() {
        String::new()
    } else {
        generate_preview(&content)
    };
    let modified_at = metadata.and_then(modified_secs).or_else(|| {
        fs::metadata(path)
            .ok()
            .and_then(|value| value.modified().ok())
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map(|value| value.as_secs())
    });
    let size = metadata
        .map(|value| value.len())
        .or_else(|| fs::metadata(path).ok().map(|v| v.len()));
    Some(NoteSummaryPayload {
        id,
        title,
        preview,
        path: path.to_string_lossy().to_string(),
        modified_at,
        size,
    })
}

fn normalize_create_title(explicit_title: Option<&str>, content: &str) -> String {
    if let Some(raw_title) = explicit_title {
        let trimmed = raw_title.trim();
        if !trimmed.is_empty() {
            return trimmed.chars().take(MAX_TITLE_CHARS).collect();
        }
    }
    let inferred = extract_title(content);
    if inferred == DEFAULT_NOTE_TITLE {
        DEFAULT_NOTE_TITLE.to_string()
    } else {
        inferred
    }
}

fn normalize_optional_folder(raw_folder: Option<&str>) -> CoreResult<String> {
    let Some(raw_folder) = raw_folder else {
        return Ok(String::new());
    };
    let trimmed = raw_folder.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let segments = normalize_id_segments(trimmed, "folder")?;
    Ok(segments.join("/"))
}

fn normalize_id_segments(raw: &str, field: &str) -> CoreResult<Vec<String>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CoreError::InvalidInput(format!(
            "{field} must not be empty"
        )));
    }
    if trimmed.contains('\\') {
        return Err(CoreError::InvalidInput(format!(
            "{field} must use forward slash separators"
        )));
    }

    let mut segments = Vec::new();
    for part in trimmed.split('/') {
        let segment = part.trim();
        if segment.is_empty() {
            return Err(CoreError::InvalidInput(format!(
                "{field} contains an empty path segment"
            )));
        }
        if segment == "." || segment == ".." {
            return Err(CoreError::InvalidInput(format!(
                "{field} must not contain relative path segments"
            )));
        }
        if segment.starts_with('.') {
            return Err(CoreError::InvalidInput(format!(
                "{field} must not contain hidden path segments"
            )));
        }
        segments.push(segment.to_string());
    }

    Ok(segments)
}

fn should_skip_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.starts_with('.') || value == "assets")
        .unwrap_or(false)
}

fn next_available_note_id(notes_root: &Path, folder: &str, base_slug: &str) -> CoreResult<String> {
    let mut candidate = if folder.is_empty() {
        base_slug.to_string()
    } else {
        format!("{folder}/{base_slug}")
    };
    let mut suffix = 1usize;

    loop {
        let path = note_path_from_id(notes_root, &candidate)?;
        if !path.exists() {
            return Ok(candidate);
        }
        candidate = if folder.is_empty() {
            format!("{base_slug}-{suffix}")
        } else {
            format!("{folder}/{base_slug}-{suffix}")
        };
        suffix += 1;
    }
}

fn cleanup_empty_note_dirs(notes_root: &Path, start: Option<&Path>) {
    let mut current = start.map(|value| value.to_path_buf());
    while let Some(dir) = current {
        if dir == notes_root {
            break;
        }
        let is_empty = fs::read_dir(&dir)
            .ok()
            .and_then(|mut iter| iter.next())
            .is_none();
        if !is_empty {
            break;
        }
        if fs::remove_dir(&dir).is_err() {
            break;
        }
        current = dir.parent().map(|value| value.to_path_buf());
    }
}

fn fallback_title_from_id(id: &str) -> String {
    id.rsplit('/')
        .next()
        .map(|value| value.replace('-', " "))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_NOTE_TITLE.to_string())
}

fn modified_secs(metadata: &fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_secs())
}

fn extract_title(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(title) = trimmed.strip_prefix("# ") {
            let title = title.trim();
            if !title.is_empty() {
                return title.chars().take(MAX_TITLE_CHARS).collect();
            }
        }
        return trimmed.chars().take(MAX_TITLE_CHARS).collect();
    }
    DEFAULT_NOTE_TITLE.to_string()
}

fn generate_preview(content: &str) -> String {
    let mut non_empty_lines = content.lines().filter_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });

    let _ = non_empty_lines.next();
    if let Some(next) = non_empty_lines.next() {
        return next.chars().take(MAX_PREVIEW_CHARS).collect();
    }
    String::new()
}
