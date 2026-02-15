use std::any::Any;
use std::sync::Arc;

use serde_json::json;

use crate::error::{CoreError, CoreResult};
use crate::extension::{
    boxed_tool_future, Extension, ExtensionInitContext, ExtensionKind, ExtensionTool,
};

use filesystem::FileSystemIndexManager;

use super::ops::{
    build_search_payload, create_note, delete_note, ensure_notes_dir, list_notes,
    notes_index_cache_dir, notes_root, read_note, update_note,
};
use super::types::NoteIndexStatusPayload;

pub struct NoteExtension {
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
        Self {
            index_manager: Arc::new(FileSystemIndexManager::default()),
        }
    }
}

#[async_trait::async_trait]
impl Extension for NoteExtension {
    fn id(&self) -> &str {
        "notes"
    }

    fn name(&self) -> &str {
        "Notes"
    }

    fn kind(&self) -> ExtensionKind {
        ExtensionKind::BuiltIn
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "notes".to_string(),
            "markdown".to_string(),
            "workspace".to_string(),
            "search".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
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
        let create_execute = Arc::new(
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
        );

        let list_execute = Arc::new(
            |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                boxed_tool_future(async move {
                    let limit = bounded_usize(&input, "limit", 50, 1, 500)?;
                    let workspace_dir = context.workspace.workspace_dir.clone();
                    let notes_root = notes_root(&workspace_dir);
                    let payload =
                        run_blocking("notes_list_notes", move || list_notes(notes_root, limit))
                            .await?;
                    Ok(json!(payload))
                })
            },
        );

        let read_execute = Arc::new(
            |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                boxed_tool_future(async move {
                    let id = required_string(&input, "id")?;
                    let workspace_dir = context.workspace.workspace_dir.clone();
                    let notes_root = notes_root(&workspace_dir);
                    let payload =
                        run_blocking("notes_read_note", move || read_note(notes_root, id)).await?;
                    Ok(json!(payload))
                })
            },
        );

        let update_execute = Arc::new(
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
        );

        let delete_execute = Arc::new(
            |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                boxed_tool_future(async move {
                    let id = required_string(&input, "id")?;
                    let workspace_dir = context.workspace.workspace_dir.clone();
                    let notes_root = notes_root(&workspace_dir);
                    let deleted =
                        run_blocking("notes_delete_note", move || delete_note(notes_root, id))
                            .await?;
                    Ok(json!({
                        "status": "ok",
                        "deleted": deleted,
                    }))
                })
            },
        );

        let index_manager = self.index_manager.clone();
        let search_execute = Arc::new(
            move |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                let index_manager = index_manager.clone();
                boxed_tool_future(async move {
                    let query = required_query(&input)?;
                    let include_hidden = optional_bool(&input, "includeHidden").unwrap_or(false);
                    let case_sensitive = optional_bool(&input, "caseSensitive").unwrap_or(false);
                    let max_results = bounded_usize(&input, "maxResults", 20, 1, 200)?;
                    let max_depth = optional_usize(&input, "maxDepth")?.unwrap_or(usize::MAX);
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
                        Ok(build_search_payload(&notes_root_for_payload, query, result))
                    })
                    .await?;
                    Ok(json!(payload))
                })
            },
        );

        let index_manager = self.index_manager.clone();
        let index_status_execute = Arc::new(
            move |_input: serde_json::Value, context: crate::extension::ExtensionContext| {
                let index_manager = index_manager.clone();
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
        );

        let index_manager = self.index_manager.clone();
        let rescan_index_execute = Arc::new(
            move |_input: serde_json::Value, context: crate::extension::ExtensionContext| {
                let index_manager = index_manager.clone();
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
        );

        vec![
            ExtensionTool {
                id: "create-note".to_string(),
                name: "Create Note".to_string(),
                description: Some(
                    "Create a note in the workspace notes directory. Default format is markdown."
                        .to_string(),
                ),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "title": { "type": "string" },
                        "content": { "type": "string" },
                        "folder": { "type": "string", "description": "Optional nested folder under notes/, using forward slashes." }
                    },
                    "additionalProperties": false
                }),
                execute: create_execute,
            },
            ExtensionTool {
                id: "list-notes".to_string(),
                name: "List Notes".to_string(),
                description: Some("List notes sorted by last modified time.".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "minimum": 1, "maximum": 500, "default": 50 }
                    },
                    "additionalProperties": false
                }),
                execute: list_execute,
            },
            ExtensionTool {
                id: "read-note".to_string(),
                name: "Read Note".to_string(),
                description: Some("Read a note by id.".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                }),
                execute: read_execute,
            },
            ExtensionTool {
                id: "update-note".to_string(),
                name: "Update Note".to_string(),
                description: Some("Replace note content by id.".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "content": { "type": "string" }
                    },
                    "required": ["id", "content"],
                    "additionalProperties": false
                }),
                execute: update_execute,
            },
            ExtensionTool {
                id: "delete-note".to_string(),
                name: "Delete Note".to_string(),
                description: Some("Delete a note by id.".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                }),
                execute: delete_execute,
            },
            ExtensionTool {
                id: "search-notes".to_string(),
                name: "Search Notes".to_string(),
                description: Some(
                    "Search notes using the shared filesystem index over workspace notes/."
                        .to_string(),
                ),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "includeHidden": { "type": "boolean", "default": false },
                        "caseSensitive": { "type": "boolean", "default": false },
                        "maxResults": { "type": "integer", "minimum": 1, "maximum": 200, "default": 20 },
                        "maxDepth": { "type": "integer", "minimum": 0 }
                    },
                    "required": ["query"],
                    "additionalProperties": false
                }),
                execute: search_execute,
            },
            ExtensionTool {
                id: "index-status".to_string(),
                name: "Index Status".to_string(),
                description: Some("Inspect notes index status.".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
                execute: index_status_execute,
            },
            ExtensionTool {
                id: "rescan-index".to_string(),
                name: "Rescan Index".to_string(),
                description: Some("Force a notes index rebuild.".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
                execute: rescan_index_execute,
            },
        ]
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
