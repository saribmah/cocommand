//! FileSystemExtension implementation.

use std::any::Any;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::json;

use crate::error::{CoreError, CoreResult};
use crate::extension::{boxed_tool_future, Extension, ExtensionInitContext, ExtensionKind, ExtensionTool};
use crate::workspace::FileSystemPreferences;

use filesystem::FileSystemIndexManager;

use super::icons;
use super::ops::{list_directory_entries, path_info, read_file_content};
use super::platform::{open_path_native, reveal_path_native};
use super::types::{EntryKindFilter, ListSortKey, SortOrder};

#[derive(Debug, Clone, Copy)]
pub(super) struct SearchRequestOptions {
    pub(super) include_hidden: bool,
    pub(super) case_sensitive: bool,
    pub(super) max_results: usize,
    pub(super) max_depth: usize,
}

#[derive(Debug, Clone)]
struct RuntimeFileSystemDefaults {
    watch_root: String,
    ignore_paths: Vec<String>,
}

pub struct FileSystemExtension {
    index_manager: Arc<FileSystemIndexManager>,
}

impl std::fmt::Debug for FileSystemExtension {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("FileSystemExtension").finish()
    }
}

impl Default for FileSystemExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystemExtension {
    pub fn new() -> Self {
        Self {
            index_manager: Arc::new(FileSystemIndexManager::default()),
        }
    }

    /// Returns a reference to the index manager.
    pub fn index_manager(&self) -> &Arc<FileSystemIndexManager> {
        &self.index_manager
    }
}

#[async_trait::async_trait]
impl Extension for FileSystemExtension {
    fn id(&self) -> &str {
        "filesystem"
    }

    fn name(&self) -> &str {
        "File System"
    }

    fn kind(&self) -> ExtensionKind {
        ExtensionKind::System
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "filesystem".to_string(),
            "files".to_string(),
            "folders".to_string(),
            "search".to_string(),
            "io".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn initialize(&self, context: ExtensionInitContext) -> CoreResult<()> {
        let defaults = workspace_filesystem_defaults(context.workspace.as_ref()).await;
        let workspace_dir = context.workspace.workspace_dir.clone();

        let root = normalize_input_path(&defaults.watch_root, &workspace_dir)?;
        let ignore_paths = normalize_ignore_paths(&defaults.ignore_paths, &root)?;
        let index_cache_dir = workspace_dir.join("storage/filesystem-indexes");

        let index_manager = self.index_manager.clone();

        // Start indexing in background thread
        tokio::task::spawn_blocking(move || {
            // index_status triggers ensure_build_started internally
            let _ = index_manager.index_status(root, index_cache_dir, ignore_paths);
        });

        Ok(())
    }

    fn tools(&self) -> Vec<ExtensionTool> {
        let index_manager = self.index_manager.clone();
        let search_execute = Arc::new(
            move |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                let index_manager = index_manager.clone();
                boxed_tool_future(async move {
                    let defaults = workspace_filesystem_defaults(context.workspace.as_ref()).await;
                    let query = required_string(&input, "query")?;
                    let root_raw =
                        optional_string(&input, "root").unwrap_or(defaults.watch_root.clone());
                    let kind = EntryKindFilter::parse(optional_string_ref(&input, "kind"))?;
                    let search_options = parse_search_request_options(&input)?;
                    let workspace_dir = context.workspace.workspace_dir.clone();

                    let root = normalize_input_path(&root_raw, &workspace_dir)?;
                    let ignore_paths = parse_ignore_paths(&input, &root, &defaults.ignore_paths)?;
                    let index_cache_dir = workspace_dir.join("storage/filesystem-indexes");
                    let fs_kind: filesystem::KindFilter = kind.into();
                    let result = run_blocking("filesystem_search_indexed", move || {
                        index_manager
                            .search(
                                root,
                                query,
                                fs_kind,
                                search_options.include_hidden,
                                search_options.case_sensitive,
                                search_options.max_results,
                                search_options.max_depth,
                                index_cache_dir,
                                ignore_paths,
                            )
                            .map_err(CoreError::from)
                    })
                    .await?;
                    let payload: super::types::SearchPayload = result.into();
                    Ok(json!(payload))
                })
            },
        );

        let list_execute = Arc::new(
            move |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                boxed_tool_future(async move {
                    let defaults = workspace_filesystem_defaults(context.workspace.as_ref()).await;
                    let path_raw =
                        optional_string(&input, "path").unwrap_or(defaults.watch_root.clone());
                    let recursive = optional_bool(&input, "recursive").unwrap_or(false);
                    let include_hidden = optional_bool(&input, "includeHidden").unwrap_or(false);
                    let kind = EntryKindFilter::parse(optional_string_ref(&input, "kind"))?;
                    let max_results = bounded_usize(&input, "maxResults", 200, 1, 2_000)?;
                    let max_depth = bounded_usize(&input, "maxDepth", 8, 0, 64)?;
                    let sort_key = ListSortKey::parse(optional_string_ref(&input, "sortBy"))?;
                    let sort_order = SortOrder::parse(optional_string_ref(&input, "sortOrder"))?;
                    let workspace_dir = context.workspace.workspace_dir.clone();

                    let path = normalize_input_path(&path_raw, &workspace_dir)?;
                    let ignore_paths = parse_ignore_paths(&input, &path, &defaults.ignore_paths)?;
                    let payload = run_blocking("filesystem_list", move || {
                        list_directory_entries(
                            path,
                            recursive,
                            include_hidden,
                            ignore_paths,
                            kind,
                            max_results,
                            max_depth,
                            sort_key,
                            sort_order,
                        )
                    })
                    .await?;
                    Ok(json!(payload))
                })
            },
        );

        let index_manager = self.index_manager.clone();
        let index_status_execute = Arc::new(
            move |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                let index_manager = index_manager.clone();
                boxed_tool_future(async move {
                    let defaults = workspace_filesystem_defaults(context.workspace.as_ref()).await;
                    let root_raw =
                        optional_string(&input, "root").unwrap_or(defaults.watch_root.clone());
                    let workspace_dir = context.workspace.workspace_dir.clone();
                    let root = normalize_input_path(&root_raw, &workspace_dir)?;
                    let ignore_paths = parse_ignore_paths(&input, &root, &defaults.ignore_paths)?;
                    let index_cache_dir = workspace_dir.join("storage/filesystem-indexes");

                    let result = run_blocking("filesystem_index_status", move || {
                        index_manager
                            .index_status(root, index_cache_dir, ignore_paths)
                            .map_err(CoreError::from)
                    })
                    .await?;
                    let payload: super::types::IndexStatusPayload = result.into();
                    Ok(json!(payload))
                })
            },
        );

        let index_manager = self.index_manager.clone();
        let rescan_index_execute = Arc::new(
            move |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                let index_manager = index_manager.clone();
                boxed_tool_future(async move {
                    let defaults = workspace_filesystem_defaults(context.workspace.as_ref()).await;
                    let root_raw =
                        optional_string(&input, "root").unwrap_or(defaults.watch_root.clone());
                    let workspace_dir = context.workspace.workspace_dir.clone();
                    let root = normalize_input_path(&root_raw, &workspace_dir)?;
                    let ignore_paths = parse_ignore_paths(&input, &root, &defaults.ignore_paths)?;
                    let index_cache_dir = workspace_dir.join("storage/filesystem-indexes");

                    let result = run_blocking("filesystem_rescan_index", move || {
                        index_manager
                            .rescan(root, index_cache_dir, ignore_paths)
                            .map_err(CoreError::from)
                    })
                    .await?;
                    let payload: super::types::IndexStatusPayload = result.into();
                    Ok(json!({
                        "status": "ok",
                        "rescanned": true,
                        "index": payload,
                    }))
                })
            },
        );

        let read_execute = Arc::new(
            |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                boxed_tool_future(async move {
                    let path_raw = required_string(&input, "path")?;
                    let offset = bounded_u64(&input, "offset", 0, 0, u64::MAX)?;
                    let max_bytes = bounded_usize(&input, "maxBytes", 16_384, 1, 1_048_576)?;
                    let workspace_dir = context.workspace.workspace_dir.clone();

                    let path = normalize_input_path(&path_raw, &workspace_dir)?;
                    let payload = run_blocking("filesystem_read_file", move || {
                        read_file_content(path, offset, max_bytes)
                    })
                    .await?;
                    Ok(json!(payload))
                })
            },
        );

        let info_execute = Arc::new(
            |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                boxed_tool_future(async move {
                    let path_raw = required_string(&input, "path")?;
                    let workspace_dir = context.workspace.workspace_dir.clone();
                    let path = normalize_input_path(&path_raw, &workspace_dir)?;

                    let payload =
                        run_blocking("filesystem_path_info", move || path_info(path)).await?;
                    Ok(json!(payload))
                })
            },
        );

        let open_execute = Arc::new(
            |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                boxed_tool_future(async move {
                    let path_raw = required_string(&input, "path")?;
                    let workspace_dir = context.workspace.workspace_dir.clone();
                    let path = normalize_input_path(&path_raw, &workspace_dir)?;
                    if !path.exists() {
                        return Err(CoreError::InvalidInput(format!(
                            "path does not exist: {}",
                            path.display()
                        )));
                    }

                    run_blocking("filesystem_open_path", move || {
                        open_path_native(&path)?;
                        Ok(json!({
                            "status": "ok",
                            "path": path.to_string_lossy(),
                        }))
                    })
                    .await
                })
            },
        );

        let reveal_execute = Arc::new(
            |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                boxed_tool_future(async move {
                    let path_raw = required_string(&input, "path")?;
                    let workspace_dir = context.workspace.workspace_dir.clone();
                    let path = normalize_input_path(&path_raw, &workspace_dir)?;
                    if !path.exists() {
                        return Err(CoreError::InvalidInput(format!(
                            "path does not exist: {}",
                            path.display()
                        )));
                    }

                    run_blocking("filesystem_reveal_path", move || {
                        reveal_path_native(&path)?;
                        Ok(json!({
                            "status": "ok",
                            "path": path.to_string_lossy(),
                        }))
                    })
                    .await
                })
            },
        );

        vec![
            ExtensionTool {
                id: "search".to_string(),
                name: "Search Files".to_string(),
                description: Some(
                    "Search files and folders with boolean expressions, ()/< > groups, wildcard/path segments (*, ?, **, slash modes), filters (ext:, type:, size:, file:, folder:, parent:, in:/infolder:, nosubfolders:), and macros (audio:, video:, doc:, exe:)."
                        .to_string(),
                ),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Supports text terms, quoted phrases, AND/OR/NOT, groups with () or <>, wildcard/path segments (*, ?, ** with slash-sensitive matching), filters ext:/type:/size:/file:/folder:/parent:/in:/infolder:/nosubfolders:, and macros audio:/video:/doc:/exe:." },
                        "root": { "type": "string", "description": "Search root path (absolute, ~, or workspace-relative)." },
                        "ignorePaths": { "type": "array", "items": { "type": "string" }, "description": "Optional paths to exclude from indexing/search (absolute, ~, or root-relative)." },
                        "kind": { "type": "string", "enum": ["all", "file", "directory"], "default": "all" },
                        "includeHidden": { "type": "boolean", "default": true },
                        "caseSensitive": { "type": "boolean", "default": false },
                        "maxResults": { "type": "integer", "minimum": 1, "maximum": 500, "default": 50 },
                        "maxDepth": { "type": "integer", "minimum": 0, "description": "Optional maximum depth from root; omit to search all indexed descendants." }
                    },
                    "required": ["query"],
                    "additionalProperties": false
                }),
                execute: search_execute,
            },
            ExtensionTool {
                id: "list_directory".to_string(),
                name: "List Directory".to_string(),
                description: Some("List files and folders under a directory.".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Directory path (absolute, ~, or workspace-relative). Defaults to configured watch_root." },
                        "ignorePaths": { "type": "array", "items": { "type": "string" }, "description": "Optional paths to exclude from listing recursion (absolute, ~, or path-relative)." },
                        "recursive": { "type": "boolean", "default": false },
                        "includeHidden": { "type": "boolean", "default": false },
                        "kind": { "type": "string", "enum": ["all", "file", "directory"], "default": "all" },
                        "maxResults": { "type": "integer", "minimum": 1, "maximum": 2000, "default": 200 },
                        "maxDepth": { "type": "integer", "minimum": 0, "maximum": 64, "default": 8 },
                        "sortBy": { "type": "string", "enum": ["name", "path", "modified", "size", "type"], "default": "name" },
                        "sortOrder": { "type": "string", "enum": ["asc", "desc"], "default": "asc" }
                    },
                    "additionalProperties": false
                }),
                execute: list_execute,
            },
            ExtensionTool {
                id: "index_status".to_string(),
                name: "Index Status".to_string(),
                description: Some(
                    "Inspect filesystem index state for a root and ignore set.".to_string(),
                ),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "root": { "type": "string", "description": "Index root path (absolute, ~, or workspace-relative). Defaults to configured watch_root." },
                        "ignorePaths": { "type": "array", "items": { "type": "string" }, "description": "Optional paths to exclude from index (absolute, ~, or root-relative). Defaults to configured ignore_paths." }
                    },
                    "additionalProperties": false
                }),
                execute: index_status_execute,
            },
            ExtensionTool {
                id: "rescan_index".to_string(),
                name: "Rescan Index".to_string(),
                description: Some(
                    "Force a full rebuild of the filesystem index for a root.".to_string(),
                ),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "root": { "type": "string", "description": "Index root path (absolute, ~, or workspace-relative). Defaults to configured watch_root." },
                        "ignorePaths": { "type": "array", "items": { "type": "string" }, "description": "Optional paths to exclude from index (absolute, ~, or root-relative). Defaults to configured ignore_paths." }
                    },
                    "additionalProperties": false
                }),
                execute: rescan_index_execute,
            },
            ExtensionTool {
                id: "read_file".to_string(),
                name: "Read File".to_string(),
                description: Some(
                    "Read UTF-8 text from a file with optional offset and byte limit.".to_string(),
                ),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "offset": { "type": "integer", "minimum": 0, "default": 0 },
                        "maxBytes": { "type": "integer", "minimum": 1, "maximum": 1048576, "default": 16384 }
                    },
                    "required": ["path"],
                    "additionalProperties": false
                }),
                execute: read_execute,
            },
            ExtensionTool {
                id: "path_info".to_string(),
                name: "Path Info".to_string(),
                description: Some("Get metadata for a file or folder path.".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    },
                    "required": ["path"],
                    "additionalProperties": false
                }),
                execute: info_execute,
            },
            ExtensionTool {
                id: "open_path".to_string(),
                name: "Open Path".to_string(),
                description: Some(
                    "Open a file or folder using the OS default handler.".to_string(),
                ),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    },
                    "required": ["path"],
                    "additionalProperties": false
                }),
                execute: open_execute,
            },
            ExtensionTool {
                id: "reveal_path".to_string(),
                name: "Reveal Path".to_string(),
                description: Some(
                    "Reveal a file in Finder (macOS) or open its parent directory.".to_string(),
                ),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    },
                    "required": ["path"],
                    "additionalProperties": false
                }),
                execute: reveal_execute,
            },
            ExtensionTool {
                id: "get_icons".to_string(),
                name: "Get Icons".to_string(),
                description: Some(
                    "Extract file/folder icons for a list of paths. Returns base64-encoded PNG data URIs. On macOS, uses NSWorkspace for system icons.".to_string(),
                ),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "paths": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "List of file paths to get icons for."
                        }
                    },
                    "required": ["paths"],
                    "additionalProperties": false
                }),
                execute: Arc::new(
                    |input: serde_json::Value, _context: crate::extension::ExtensionContext| {
                        boxed_tool_future(async move {
                            let paths_value = input.get("paths").ok_or_else(|| {
                                CoreError::InvalidInput("missing paths".to_string())
                            })?;
                            let paths_array = paths_value.as_array().ok_or_else(|| {
                                CoreError::InvalidInput("paths must be an array".to_string())
                            })?;
                            let paths: Vec<String> = paths_array
                                .iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect();

                            let payload = run_blocking("filesystem_get_icons", move || {
                                Ok(icons::extract_icons(paths))
                            })
                            .await?;
                            Ok(json!(payload))
                        })
                    },
                ),
            },
        ]
    }
}

fn optional_string_ref<'a>(input: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    input.get(key).and_then(|value| value.as_str())
}

fn optional_string(input: &serde_json::Value, key: &str) -> Option<String> {
    optional_string_ref(input, key).map(|value| value.to_string())
}

fn optional_bool(input: &serde_json::Value, key: &str) -> Option<bool> {
    input.get(key).and_then(|value| value.as_bool())
}

async fn workspace_filesystem_defaults(
    workspace: &crate::workspace::WorkspaceInstance,
) -> RuntimeFileSystemDefaults {
    let config = workspace.config.read().await;
    let FileSystemPreferences {
        watch_root,
        ignore_paths,
    } = config.preferences.filesystem.clone();
    RuntimeFileSystemDefaults {
        watch_root: if watch_root.trim().is_empty() {
            "~".to_string()
        } else {
            watch_root
        },
        ignore_paths,
    }
}

pub(super) fn parse_ignore_paths(
    input: &serde_json::Value,
    root: &Path,
    default_paths: &[String],
) -> CoreResult<Vec<PathBuf>> {
    let raw_paths = if let Some(raw_value) = input.get("ignorePaths") {
        let raw_array = raw_value.as_array().ok_or_else(|| {
            CoreError::InvalidInput("ignorePaths must be an array of strings".to_string())
        })?;
        let mut parsed = Vec::with_capacity(raw_array.len());
        for raw in raw_array {
            let value = raw
                .as_str()
                .map(|candidate| candidate.trim().to_string())
                .filter(|candidate| !candidate.is_empty())
                .ok_or_else(|| {
                    CoreError::InvalidInput(
                        "ignorePaths must contain non-empty strings".to_string(),
                    )
                })?;
            parsed.push(value);
        }
        parsed
    } else {
        default_paths.to_vec()
    };

    normalize_ignore_paths(&raw_paths, root)
}

fn normalize_ignore_paths(raw_paths: &[String], root: &Path) -> CoreResult<Vec<PathBuf>> {
    let mut normalized = Vec::new();
    for raw_path in raw_paths {
        if raw_path.trim().is_empty() {
            continue;
        }
        let candidate =
            if raw_path == "~" || raw_path.starts_with("~/") || raw_path.starts_with("~\\") {
                expand_home_path(raw_path)?
            } else {
                let path = PathBuf::from(raw_path);
                if path.is_absolute() {
                    path
                } else {
                    root.join(path)
                }
            };
        normalized.push(canonicalize_existing_path(candidate));
    }

    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

pub(super) fn required_string(input: &serde_json::Value, key: &str) -> CoreResult<String> {
    let value = input
        .get(key)
        .and_then(|raw| raw.as_str())
        .map(|raw| raw.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| CoreError::InvalidInput(format!("missing {key}")))?;
    Ok(value.to_string())
}

pub(super) fn parse_search_request_options(input: &serde_json::Value) -> CoreResult<SearchRequestOptions> {
    let include_hidden = optional_bool(input, "includeHidden").unwrap_or(true);
    let case_sensitive = optional_bool(input, "caseSensitive").unwrap_or(false);
    let max_results = bounded_usize(input, "maxResults", 50, 1, 500)?;
    let max_depth = optional_usize(input, "maxDepth")?.unwrap_or(usize::MAX);

    Ok(SearchRequestOptions {
        include_hidden,
        case_sensitive,
        max_results,
        max_depth,
    })
}

fn bounded_usize(
    input: &serde_json::Value,
    key: &str,
    default: usize,
    min: usize,
    max: usize,
) -> CoreResult<usize> {
    let value = match input.get(key) {
        Some(raw) => raw
            .as_u64()
            .ok_or_else(|| CoreError::InvalidInput(format!("{key} must be an integer")))?
            as usize,
        None => default,
    };
    Ok(value.clamp(min, max))
}

fn optional_usize(input: &serde_json::Value, key: &str) -> CoreResult<Option<usize>> {
    match input.get(key) {
        Some(raw) => {
            let value = raw
                .as_u64()
                .ok_or_else(|| CoreError::InvalidInput(format!("{key} must be an integer")))?;
            let parsed = usize::try_from(value)
                .map_err(|_| CoreError::InvalidInput(format!("{key} is too large")))?;
            Ok(Some(parsed))
        }
        None => Ok(None),
    }
}

fn bounded_u64(
    input: &serde_json::Value,
    key: &str,
    default: u64,
    min: u64,
    max: u64,
) -> CoreResult<u64> {
    let value = match input.get(key) {
        Some(raw) => raw
            .as_u64()
            .ok_or_else(|| CoreError::InvalidInput(format!("{key} must be an integer")))?,
        None => default,
    };
    Ok(value.clamp(min, max))
}

async fn run_blocking<F, T>(task_name: &str, task: F) -> CoreResult<T>
where
    F: FnOnce() -> CoreResult<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|error| CoreError::Internal(format!("{task_name} task failed: {error}")))?
}

pub(super) fn normalize_input_path(raw: &str, workspace_dir: &Path) -> CoreResult<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CoreError::InvalidInput(
            "path must not be empty".to_string(),
        ));
    }

    let candidate = if trimmed == "~" || trimmed.starts_with("~/") || trimmed.starts_with("~\\") {
        expand_home_path(trimmed)?
    } else {
        PathBuf::from(trimmed)
    };

    if candidate.is_absolute() {
        Ok(candidate)
    } else {
        Ok(workspace_dir.join(candidate))
    }
}

fn expand_home_path(raw: &str) -> CoreResult<PathBuf> {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| CoreError::Internal("HOME is not set".to_string()))?;
    if raw == "~" {
        return Ok(home);
    }
    let rest = raw
        .strip_prefix("~/")
        .or_else(|| raw.strip_prefix("~\\"))
        .unwrap_or_default();
    Ok(home.join(rest))
}

pub(super) fn canonicalize_existing_path(path: PathBuf) -> PathBuf {
    fs::canonicalize(&path).unwrap_or(path)
}
