//! FileSystemExtension implementation.

use std::any::Any;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::json;

use crate::error::{CoreError, CoreResult};
use crate::extension::builtin::manifest_tools::{merge_manifest_tools, parse_builtin_manifest};
use crate::extension::manifest::ExtensionManifest;
use crate::extension::{
    boxed_tool_future, Extension, ExtensionInitContext, ExtensionKind, ExtensionStatus,
    ExtensionTool,
};
use crate::workspace::FileSystemPreferences;

use filesystem::indexer::IndexBuildState;
use filesystem::FileSystemIndexManager;

use super::icons;
use super::ops::{list_directory_entries, path_info, read_file_content};
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
    manifest: ExtensionManifest,
    tools: Vec<ExtensionTool>,
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
        let manifest = parse_builtin_manifest(include_str!("manifest.json"));
        let index_manager = Arc::new(FileSystemIndexManager::default());

        let mut execute_map = HashMap::new();

        let im = index_manager.clone();
        execute_map.insert(
            "search",
            Arc::new(
                move |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    let index_manager = im.clone();
                    boxed_tool_future(async move {
                        let defaults =
                            workspace_filesystem_defaults(context.workspace.as_ref()).await;
                        let query = required_string(&input, "query")?;
                        let root_raw =
                            optional_string(&input, "root").unwrap_or(defaults.watch_root.clone());
                        let kind = EntryKindFilter::parse(optional_string_ref(&input, "kind"))?;
                        let search_options = parse_search_request_options(&input)?;
                        let workspace_dir = context.workspace.workspace_dir.clone();

                        let root = normalize_input_path(&root_raw, &workspace_dir)?;
                        let ignore_paths =
                            parse_ignore_paths(&input, &root, &defaults.ignore_paths)?;
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
                                    None,
                                )
                                .map_err(CoreError::from)
                        })
                        .await?;
                        let payload: super::types::SearchPayload = result.unwrap().into();
                        Ok(json!(payload))
                    })
                },
            ) as _,
        );

        execute_map.insert(
            "list_directory",
            Arc::new(
                move |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let defaults =
                            workspace_filesystem_defaults(context.workspace.as_ref()).await;
                        let path_raw =
                            optional_string(&input, "path").unwrap_or(defaults.watch_root.clone());
                        let recursive = optional_bool(&input, "recursive").unwrap_or(false);
                        let include_hidden =
                            optional_bool(&input, "includeHidden").unwrap_or(false);
                        let kind = EntryKindFilter::parse(optional_string_ref(&input, "kind"))?;
                        let max_results = bounded_usize(&input, "maxResults", 200, 1, 2_000)?;
                        let max_depth = bounded_usize(&input, "maxDepth", 8, 0, 64)?;
                        let sort_key = ListSortKey::parse(optional_string_ref(&input, "sortBy"))?;
                        let sort_order =
                            SortOrder::parse(optional_string_ref(&input, "sortOrder"))?;
                        let workspace_dir = context.workspace.workspace_dir.clone();

                        let path = normalize_input_path(&path_raw, &workspace_dir)?;
                        let ignore_paths =
                            parse_ignore_paths(&input, &path, &defaults.ignore_paths)?;
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
            ) as _,
        );

        let im = index_manager.clone();
        execute_map.insert(
            "index_status",
            Arc::new(
                move |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    let index_manager = im.clone();
                    boxed_tool_future(async move {
                        let defaults =
                            workspace_filesystem_defaults(context.workspace.as_ref()).await;
                        let root_raw =
                            optional_string(&input, "root").unwrap_or(defaults.watch_root.clone());
                        let workspace_dir = context.workspace.workspace_dir.clone();
                        let root = normalize_input_path(&root_raw, &workspace_dir)?;
                        let ignore_paths =
                            parse_ignore_paths(&input, &root, &defaults.ignore_paths)?;
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
            ) as _,
        );

        let im = index_manager.clone();
        execute_map.insert(
            "rescan_index",
            Arc::new(
                move |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    let index_manager = im.clone();
                    boxed_tool_future(async move {
                        let defaults =
                            workspace_filesystem_defaults(context.workspace.as_ref()).await;
                        let root_raw =
                            optional_string(&input, "root").unwrap_or(defaults.watch_root.clone());
                        let workspace_dir = context.workspace.workspace_dir.clone();
                        let root = normalize_input_path(&root_raw, &workspace_dir)?;
                        let ignore_paths =
                            parse_ignore_paths(&input, &root, &defaults.ignore_paths)?;
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
            ) as _,
        );

        execute_map.insert(
            "read_file",
            Arc::new(
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
            ) as _,
        );

        execute_map.insert(
            "path_info",
            Arc::new(
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
            ) as _,
        );

        execute_map.insert(
            "open_path",
            Arc::new(
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
                        let platform = context.workspace.platform.clone();

                        run_blocking("filesystem_open_path", move || {
                            platform.open_path(&path)?;
                            Ok(json!({
                                "status": "ok",
                                "path": path.to_string_lossy(),
                            }))
                        })
                        .await
                    })
                },
            ) as _,
        );

        execute_map.insert(
            "reveal_path",
            Arc::new(
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
                        let platform = context.workspace.platform.clone();

                        run_blocking("filesystem_reveal_path", move || {
                            platform.reveal_path(&path)?;
                            Ok(json!({
                                "status": "ok",
                                "path": path.to_string_lossy(),
                            }))
                        })
                        .await
                    })
                },
            ) as _,
        );

        execute_map.insert(
            "get_icons",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let paths_value = input
                            .get("paths")
                            .ok_or_else(|| CoreError::InvalidInput("missing paths".to_string()))?;
                        let paths_array = paths_value.as_array().ok_or_else(|| {
                            CoreError::InvalidInput("paths must be an array".to_string())
                        })?;
                        let paths: Vec<String> = paths_array
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                        let platform = context.workspace.platform.clone();

                        let payload = run_blocking("filesystem_get_icons", move || {
                            Ok(icons::extract_icons(paths, platform))
                        })
                        .await?;
                        Ok(json!(payload))
                    })
                },
            ) as _,
        );

        let tools = merge_manifest_tools(&manifest, execute_map);

        Self {
            manifest,
            tools,
            index_manager,
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
        &self.manifest.id
    }

    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn kind(&self) -> ExtensionKind {
        ExtensionKind::System
    }

    fn tags(&self) -> Vec<String> {
        self.manifest
            .routing
            .as_ref()
            .and_then(|r| r.keywords.clone())
            .unwrap_or_default()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn status(&self) -> ExtensionStatus {
        match self.index_manager.peek_build_state() {
            IndexBuildState::Idle | IndexBuildState::Building => ExtensionStatus::Building,
            IndexBuildState::Ready | IndexBuildState::Updating => ExtensionStatus::Ready,
            IndexBuildState::Error => ExtensionStatus::Error,
        }
    }

    async fn initialize(&self, context: ExtensionInitContext) -> CoreResult<()> {
        let defaults = workspace_filesystem_defaults(context.workspace.as_ref()).await;
        let workspace_dir = context.workspace.workspace_dir.clone();

        let root = normalize_input_path(&defaults.watch_root, &workspace_dir)?;
        let ignore_paths = normalize_ignore_paths(&defaults.ignore_paths, &root)?;
        let index_cache_dir = workspace_dir.join("storage/filesystem-indexes");

        let index_manager = self.index_manager.clone();

        tokio::task::spawn_blocking(move || {
            let _ = index_manager.index_status(root, index_cache_dir, ignore_paths);
        });

        Ok(())
    }

    fn tools(&self) -> Vec<ExtensionTool> {
        self.tools.clone()
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
        .ok_or_else(|| CoreError::InvalidInput(format!("missing {key}")))?;
    let trimmed = value.trim();
    if trimmed.is_empty() && key != "query" {
        return Err(CoreError::InvalidInput(format!("missing {key}")));
    }
    Ok(trimmed.to_string())
}

pub(super) fn parse_search_request_options(
    input: &serde_json::Value,
) -> CoreResult<SearchRequestOptions> {
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
