use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;

use crate::error::{CoreError, CoreResult};
use crate::extension::builtin::manifest_tools::{merge_manifest_tools, parse_builtin_manifest};
use crate::extension::manifest::ExtensionManifest;
use crate::extension::{
    boxed_tool_future, Extension, ExtensionInitContext, ExtensionKind, ExtensionStatus,
    ExtensionTool,
};

use filesystem::indexer::IndexBuildState;
use filesystem::FileSystemIndexManager;

use super::ops::{
    build_search_payload, create_note, delete_note, ensure_notes_dir, list_notes,
    notes_index_cache_dir, notes_root, read_note, update_note,
};
use super::types::NoteIndexStatusPayload;

pub struct NoteExtension {
    manifest: ExtensionManifest,
    tools: Vec<ExtensionTool>,
    index_manager: Arc<FileSystemIndexManager>,
}

impl std::fmt::Debug for NoteExtension {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("NoteExtension").finish()
    }
}

impl Default for NoteExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl NoteExtension {
    pub fn new() -> Self {
        let manifest = parse_builtin_manifest(include_str!("manifest.json"));
        let index_manager = Arc::new(FileSystemIndexManager::default());

        let mut execute_map = HashMap::new();

        execute_map.insert(
            "create-note",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let title = optional_string(&input, "title");
                        let content = optional_string_allow_empty(&input, "content");
                        let folder = optional_string(&input, "folder");
                        let workspace_dir = context.workspace.workspace_dir.clone();
                        let notes_root = notes_root(&workspace_dir);
                        let payload = run_blocking("notes_create_note", move || {
                            create_note(notes_root, title, content, folder)
                        })
                        .await?;
                        Ok(json!(payload))
                    })
                },
            ) as _,
        );

        execute_map.insert(
            "list-notes",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let limit = bounded_usize(&input, "limit", 50, 1, 500)?;
                        let workspace_dir = context.workspace.workspace_dir.clone();
                        let notes_root = notes_root(&workspace_dir);
                        let payload = run_blocking("notes_list_notes", move || {
                            list_notes(notes_root, limit)
                        })
                        .await?;
                        Ok(json!(payload))
                    })
                },
            ) as _,
        );

        execute_map.insert(
            "read-note",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let id = required_string(&input, "id")?;
                        let workspace_dir = context.workspace.workspace_dir.clone();
                        let notes_root = notes_root(&workspace_dir);
                        let payload = run_blocking("notes_read_note", move || {
                            read_note(notes_root, id)
                        })
                        .await?;
                        Ok(json!(payload))
                    })
                },
            ) as _,
        );

        execute_map.insert(
            "update-note",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let id = required_string(&input, "id")?;
                        let content = required_raw_string(&input, "content")?;
                        let workspace_dir = context.workspace.workspace_dir.clone();
                        let notes_root = notes_root(&workspace_dir);
                        let payload = run_blocking("notes_update_note", move || {
                            update_note(notes_root, id, content)
                        })
                        .await?;
                        Ok(json!(payload))
                    })
                },
            ) as _,
        );

        execute_map.insert(
            "delete-note",
            Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let id = required_string(&input, "id")?;
                        let workspace_dir = context.workspace.workspace_dir.clone();
                        let notes_root = notes_root(&workspace_dir);
                        let deleted = run_blocking("notes_delete_note", move || {
                            delete_note(notes_root, id)
                        })
                        .await?;
                        Ok(json!({
                            "status": "ok",
                            "deleted": deleted,
                        }))
                    })
                },
            ) as _,
        );

        let im = index_manager.clone();
        execute_map.insert(
            "search-notes",
            Arc::new(
                move |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    let index_manager = im.clone();
                    boxed_tool_future(async move {
                        let query = required_query(&input)?;
                        let include_hidden =
                            optional_bool(&input, "includeHidden").unwrap_or(false);
                        let case_sensitive =
                            optional_bool(&input, "caseSensitive").unwrap_or(false);
                        let max_results = bounded_usize(&input, "maxResults", 20, 1, 200)?;
                        let max_depth =
                            optional_usize(&input, "maxDepth")?.unwrap_or(usize::MAX);
                        let workspace_dir = context.workspace.workspace_dir.clone();
                        let notes_root = notes_root(&workspace_dir);
                        let notes_root_for_payload = notes_root.clone();
                        let index_cache_dir = notes_index_cache_dir(&workspace_dir);

                        let payload = run_blocking("notes_search_notes", move || {
                            ensure_notes_dir(&notes_root)?;
                            let result = index_manager
                                .search(
                                    notes_root.clone(),
                                    query.clone(),
                                    filesystem::KindFilter::File,
                                    include_hidden,
                                    case_sensitive,
                                    max_results,
                                    max_depth,
                                    index_cache_dir,
                                    Vec::new(),
                                    None,
                                )
                                .map_err(CoreError::from)?;
                            Ok(build_search_payload(
                                &notes_root_for_payload,
                                query,
                                result,
                            ))
                        })
                        .await?;
                        Ok(json!(payload))
                    })
                },
            ) as _,
        );

        let im = index_manager.clone();
        execute_map.insert(
            "index-status",
            Arc::new(
                move |_input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    let index_manager = im.clone();
                    boxed_tool_future(async move {
                        let workspace_dir = context.workspace.workspace_dir.clone();
                        let notes_root = notes_root(&workspace_dir);
                        let index_cache_dir = notes_index_cache_dir(&workspace_dir);
                        let payload = run_blocking("notes_index_status", move || {
                            ensure_notes_dir(&notes_root)?;
                            let status = index_manager
                                .index_status(notes_root, index_cache_dir, Vec::new())
                                .map_err(CoreError::from)?;
                            Ok(NoteIndexStatusPayload::from(status))
                        })
                        .await?;
                        Ok(json!(payload))
                    })
                },
            ) as _,
        );

        let im = index_manager.clone();
        execute_map.insert(
            "rescan-index",
            Arc::new(
                move |_input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    let index_manager = im.clone();
                    boxed_tool_future(async move {
                        let workspace_dir = context.workspace.workspace_dir.clone();
                        let notes_root = notes_root(&workspace_dir);
                        let index_cache_dir = notes_index_cache_dir(&workspace_dir);
                        let payload = run_blocking("notes_rescan_index", move || {
                            ensure_notes_dir(&notes_root)?;
                            let status = index_manager
                                .rescan(notes_root, index_cache_dir, Vec::new())
                                .map_err(CoreError::from)?;
                            Ok(NoteIndexStatusPayload::from(status))
                        })
                        .await?;
                        Ok(json!({
                            "status": "ok",
                            "rescanned": true,
                            "index": payload,
                        }))
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
}

#[async_trait::async_trait]
impl Extension for NoteExtension {
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
        let workspace_dir = context.workspace.workspace_dir.clone();
        let notes_root = notes_root(&workspace_dir);
        ensure_notes_dir(&notes_root)?;

        let index_cache_dir = notes_index_cache_dir(&workspace_dir);
        let index_manager = self.index_manager.clone();

        tokio::task::spawn_blocking(move || {
            let _ = index_manager.index_status(notes_root, index_cache_dir, Vec::new());
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
    optional_string_ref(input, key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn optional_string_allow_empty(input: &serde_json::Value, key: &str) -> Option<String> {
    optional_string_ref(input, key).map(|value| value.to_string())
}

fn optional_bool(input: &serde_json::Value, key: &str) -> Option<bool> {
    input.get(key).and_then(|value| value.as_bool())
}

fn required_string(input: &serde_json::Value, key: &str) -> CoreResult<String> {
    let value = input
        .get(key)
        .and_then(|raw| raw.as_str())
        .ok_or_else(|| CoreError::InvalidInput(format!("missing {key}")))?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(CoreError::InvalidInput(format!("missing {key}")));
    }
    Ok(trimmed.to_string())
}

fn required_raw_string(input: &serde_json::Value, key: &str) -> CoreResult<String> {
    input
        .get(key)
        .and_then(|raw| raw.as_str())
        .map(|value| value.to_string())
        .ok_or_else(|| CoreError::InvalidInput(format!("missing {key}")))
}

fn required_query(input: &serde_json::Value) -> CoreResult<String> {
    let query = required_string(input, "query")?;
    if query.is_empty() {
        return Err(CoreError::InvalidInput("missing query".to_string()));
    }
    Ok(query)
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

async fn run_blocking<F, T>(task_name: &str, task: F) -> CoreResult<T>
where
    F: FnOnce() -> CoreResult<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|error| CoreError::Internal(format!("{task_name} task failed: {error}")))?
}
