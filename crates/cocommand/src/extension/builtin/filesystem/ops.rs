use std::collections::VecDeque;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use crate::error::{CoreError, CoreResult};

use super::types::{
    build_entry, is_hidden_path, modified_secs, sort_entries, EntryKindFilter, EntryType,
    ListPayload, ListSortKey, PathInfoPayload, ReadFilePayload, SortOrder,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn list_directory_entries(
    path: PathBuf,
    recursive: bool,
    include_hidden: bool,
    ignored_roots: Vec<PathBuf>,
    kind: EntryKindFilter,
    max_results: usize,
    max_depth: usize,
    sort_key: ListSortKey,
    sort_order: SortOrder,
) -> CoreResult<ListPayload> {
    let metadata = fs::symlink_metadata(&path).map_err(|error| {
        CoreError::InvalidInput(format!("unable to access path {}: {error}", path.display()))
    })?;
    if !metadata.file_type().is_dir() {
        return Err(CoreError::InvalidInput(format!(
            "path is not a directory: {}",
            path.display()
        )));
    }
    if path_is_ignored(&ignored_roots, &path) {
        return Ok(ListPayload {
            path: path.to_string_lossy().to_string(),
            recursive,
            count: 0,
            results: Vec::new(),
            truncated: false,
            errors: 0,
        });
    }

    let mut queue = VecDeque::new();
    queue.push_back((path.clone(), 0usize));

    let mut results = Vec::new();
    let mut errors = 0usize;
    let mut truncated = false;

    'outer: while let Some((current, depth)) = queue.pop_front() {
        let dir_iter = match fs::read_dir(&current) {
            Ok(iter) => iter,
            Err(_) => {
                errors += 1;
                continue;
            }
        };
        let mut children = Vec::new();
        for child in dir_iter {
            match child {
                Ok(child) => children.push(child.path()),
                Err(_) => errors += 1,
            }
        }
        children.sort();

        for child in children {
            if path_is_ignored(&ignored_roots, &child) {
                continue;
            }
            if !include_hidden && is_hidden_path(&child) {
                continue;
            }

            let metadata = match fs::symlink_metadata(&child) {
                Ok(metadata) => metadata,
                Err(_) => {
                    errors += 1;
                    continue;
                }
            };
            let file_type = metadata.file_type();
            if kind.matches(&file_type) {
                results.push(build_entry(&child, &metadata));
                if results.len() >= max_results {
                    truncated = true;
                    break 'outer;
                }
            }

            if recursive && file_type.is_dir() && depth < max_depth {
                queue.push_back((child, depth + 1));
            }
        }
    }

    sort_entries(&mut results, sort_key, sort_order);

    Ok(ListPayload {
        path: path.to_string_lossy().to_string(),
        recursive,
        count: results.len(),
        results,
        truncated,
        errors,
    })
}

fn path_is_ignored(ignored_roots: &[PathBuf], candidate: &PathBuf) -> bool {
    ignored_roots
        .iter()
        .any(|ignored| candidate == ignored || candidate.starts_with(ignored))
}

pub(super) fn read_file_content(
    path: PathBuf,
    offset: u64,
    max_bytes: usize,
) -> CoreResult<ReadFilePayload> {
    let mut file = fs::File::open(&path).map_err(|error| {
        CoreError::InvalidInput(format!("unable to open file {}: {error}", path.display()))
    })?;
    let metadata = file.metadata().map_err(|error| {
        CoreError::Internal(format!(
            "unable to read metadata for {}: {error}",
            path.display()
        ))
    })?;
    let total_bytes = metadata.len();

    file.seek(SeekFrom::Start(offset)).map_err(|error| {
        CoreError::Internal(format!(
            "unable to seek file {} at offset {}: {error}",
            path.display(),
            offset
        ))
    })?;

    let mut buffer = vec![0u8; max_bytes];
    let bytes_read = file.read(&mut buffer).map_err(|error| {
        CoreError::Internal(format!("unable to read file {}: {error}", path.display()))
    })?;
    buffer.truncate(bytes_read);
    let content = String::from_utf8_lossy(&buffer).to_string();
    let truncated = offset.saturating_add(bytes_read as u64) < total_bytes;

    Ok(ReadFilePayload {
        path: path.to_string_lossy().to_string(),
        content,
        offset,
        bytes_read,
        total_bytes,
        truncated,
    })
}

pub(super) fn path_info(path: PathBuf) -> CoreResult<PathInfoPayload> {
    if !path.exists() {
        return Ok(PathInfoPayload {
            path: path.to_string_lossy().to_string(),
            exists: false,
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().to_string()),
            parent: path
                .parent()
                .map(|value| value.to_string_lossy().to_string()),
            extension: None,
            entry_type: None,
            size: None,
            modified_at: None,
            readonly: None,
            hidden: None,
        });
    }

    let metadata = fs::symlink_metadata(&path).map_err(|error| {
        CoreError::Internal(format!(
            "unable to read metadata for {}: {error}",
            path.display()
        ))
    })?;
    let file_type = metadata.file_type();
    Ok(PathInfoPayload {
        path: path.to_string_lossy().to_string(),
        exists: true,
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().to_string()),
        parent: path
            .parent()
            .map(|value| value.to_string_lossy().to_string()),
        extension: path
            .extension()
            .map(|value| value.to_string_lossy().to_string()),
        entry_type: Some(EntryType::from_file_type(&file_type).as_str().to_string()),
        size: file_type.is_file().then_some(metadata.len()),
        modified_at: modified_secs(&metadata),
        readonly: Some(metadata.permissions().readonly()),
        hidden: Some(is_hidden_path(&path)),
    })
}
