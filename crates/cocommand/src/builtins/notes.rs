//! Notes built-in app: list, latest, create, update, delete tools.

use serde_json::json;
use uuid::Uuid;

use crate::routing::RoutingMetadata;
use crate::tools::schema::{RiskLevel, ToolDefinition};
use crate::tools::registry::ToolRegistry;
use crate::routing::Router;

/// App identifier.
pub const APP_ID: &str = "notes";

/// Key used in ApplicationInstance.context to store notes.
const NOTES_KEY: &str = "notes";

/// Register notes tools and routing metadata.
pub fn register(registry: &mut ToolRegistry, router: &mut Router) {
    registry.register_kernel_tool(list_tool());
    registry.register_kernel_tool(latest_tool());
    registry.register_kernel_tool(create_tool());
    registry.register_kernel_tool(update_tool());
    registry.register_kernel_tool(delete_tool());
    router.register(routing_metadata());
}

/// Routing metadata for the notes app.
fn routing_metadata() -> RoutingMetadata {
    RoutingMetadata {
        app_id: APP_ID.to_string(),
        keywords: vec![
            "note".into(),
            "notes".into(),
            "memo".into(),
            "write".into(),
        ],
        examples: vec![
            "create a new note".into(),
            "show my notes".into(),
            "delete last note".into(),
            "show last note".into(),
        ],
        verbs: vec![
            "create".into(),
            "list".into(),
            "show".into(),
            "update".into(),
            "delete".into(),
            "write".into(),
        ],
        objects: vec![
            "note".into(),
            "notes".into(),
            "memo".into(),
        ],
    }
}

/// Tool definition for `notes.list`.
fn list_tool() -> ToolDefinition {
    ToolDefinition {
        tool_id: "notes.list".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "limit": {"type": "integer"}
            }
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "notes": {"type": "array"},
                "count": {"type": "integer"}
            }
        }),
        risk_level: RiskLevel::Safe,
        is_kernel: false,
        handler: Box::new(|args, ctx| {
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(50) as usize;

            let notes = get_notes(ctx.workspace);
            let limited: Vec<_> = notes.iter().rev().take(limit).cloned().collect();
            let count = limited.len();

            Ok(json!({
                "notes": limited,
                "count": count
            }))
        }),
    }
}

/// Tool definition for `notes.latest`.
fn latest_tool() -> ToolDefinition {
    ToolDefinition {
        tool_id: "notes.latest".to_string(),
        input_schema: json!({
            "type": "object"
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "note": {},
                "found": {"type": "boolean"}
            }
        }),
        risk_level: RiskLevel::Safe,
        is_kernel: false,
        handler: Box::new(|_args, ctx| {
            let notes = get_notes(ctx.workspace);
            match notes.last() {
                Some(note) => Ok(json!({
                    "note": note,
                    "found": true
                })),
                None => Ok(json!({
                    "note": null,
                    "found": false
                })),
            }
        }),
    }
}

/// Tool definition for `notes.create`.
fn create_tool() -> ToolDefinition {
    ToolDefinition {
        tool_id: "notes.create".to_string(),
        input_schema: json!({
            "type": "object",
            "required": ["title", "content"],
            "properties": {
                "title": {"type": "string"},
                "content": {"type": "string"}
            }
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "id": {"type": "string"},
                "note": {"type": "object"}
            }
        }),
        risk_level: RiskLevel::Safe,
        is_kernel: false,
        handler: Box::new(|args, ctx| {
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled")
                .to_string();
            let content = args
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let id = Uuid::new_v4().to_string();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let note = json!({
                "id": id,
                "title": title,
                "content": content,
                "created_at": now
            });

            let mut notes = get_notes(ctx.workspace);
            notes.push(note.clone());
            set_notes(ctx.workspace, &notes);

            Ok(json!({
                "id": id,
                "note": note
            }))
        }),
    }
}

/// Tool definition for `notes.update`.
fn update_tool() -> ToolDefinition {
    ToolDefinition {
        tool_id: "notes.update".to_string(),
        input_schema: json!({
            "type": "object",
            "required": ["id"],
            "properties": {
                "id": {"type": "string"},
                "title": {"type": "string"},
                "content": {"type": "string"}
            }
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "note": {"type": "object"},
                "found": {"type": "boolean"}
            }
        }),
        risk_level: RiskLevel::Safe,
        is_kernel: false,
        handler: Box::new(|args, ctx| {
            let id = args
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let mut notes = get_notes(ctx.workspace);
            let pos = notes.iter().position(|n| {
                n.get("id").and_then(|v| v.as_str()) == Some(id)
            });

            match pos {
                Some(idx) => {
                    if let Some(title) = args.get("title").and_then(|v| v.as_str()) {
                        notes[idx]["title"] = json!(title);
                    }
                    if let Some(content) = args.get("content").and_then(|v| v.as_str()) {
                        notes[idx]["content"] = json!(content);
                    }
                    let updated = notes[idx].clone();
                    set_notes(ctx.workspace, &notes);
                    Ok(json!({
                        "note": updated,
                        "found": true
                    }))
                }
                None => Ok(json!({
                    "note": null,
                    "found": false
                })),
            }
        }),
    }
}

/// Tool definition for `notes.delete`. Risk level is Destructive (requires confirmation).
fn delete_tool() -> ToolDefinition {
    ToolDefinition {
        tool_id: "notes.delete".to_string(),
        input_schema: json!({
            "type": "object",
            "required": ["id"],
            "properties": {
                "id": {"type": "string"}
            }
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "deleted": {"type": "boolean"},
                "id": {"type": "string"}
            }
        }),
        risk_level: RiskLevel::Destructive,
        is_kernel: false,
        handler: Box::new(|args, ctx| {
            let id = args
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let mut notes = get_notes(ctx.workspace);
            let original_len = notes.len();
            notes.retain(|n| {
                n.get("id").and_then(|v| v.as_str()) != Some(id)
            });
            let deleted = notes.len() < original_len;
            set_notes(ctx.workspace, &notes);

            Ok(json!({
                "deleted": deleted,
                "id": id
            }))
        }),
    }
}

/// Read notes from workspace context.
fn get_notes(workspace: &crate::workspace::Workspace) -> Vec<serde_json::Value> {
    workspace
        .instances
        .get(APP_ID)
        .and_then(|inst| inst.context.get(NOTES_KEY))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

/// Write notes to workspace context, creating the instance if needed.
fn set_notes(workspace: &mut crate::workspace::Workspace, notes: &[serde_json::Value]) {
    let instance = workspace
        .instances
        .entry(APP_ID.to_string())
        .or_insert_with(|| crate::workspace::ApplicationInstance {
            instance_id: APP_ID.to_string(),
            app_id: APP_ID.to_string(),
            status: crate::workspace::ApplicationStatus::Active,
            context: std::collections::HashMap::new(),
            mounted_tools: vec![
                "notes.list".into(),
                "notes.latest".into(),
                "notes.create".into(),
                "notes.update".into(),
                "notes.delete".into(),
            ],
        });
    instance.context.insert(NOTES_KEY.to_string(), json!(notes));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::storage::{MemoryStorage, Storage};
    use crate::tools::schema::ExecutionContext;
    use crate::workspace::{ApplicationInstance, ApplicationStatus, Workspace};

    fn setup_workspace_with_notes(notes: Vec<serde_json::Value>) -> Workspace {
        let mut ws = Workspace::new("test".to_string());
        let mut context = HashMap::new();
        context.insert(NOTES_KEY.to_string(), json!(notes));
        let instance = ApplicationInstance {
            instance_id: APP_ID.to_string(),
            app_id: APP_ID.to_string(),
            status: ApplicationStatus::Active,
            context,
            mounted_tools: vec![
                "notes.list".into(),
                "notes.latest".into(),
                "notes.create".into(),
                "notes.update".into(),
                "notes.delete".into(),
            ],
        };
        ws.instances.insert(APP_ID.to_string(), instance);
        ws
    }

    fn make_note(id: &str, title: &str, content: &str) -> serde_json::Value {
        json!({
            "id": id,
            "title": title,
            "content": content,
            "created_at": 1000
        })
    }

    #[test]
    fn list_returns_empty_when_no_notes() {
        let tool = list_tool();
        let mut ws = Workspace::new("test".to_string());
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log: storage.event_log_mut(),
        };
        let result = (tool.handler)(&json!({}), &mut ctx).unwrap();
        assert_eq!(result["count"], 0);
    }

    #[test]
    fn list_returns_notes_reverse_order() {
        let tool = list_tool();
        let mut ws = setup_workspace_with_notes(vec![
            make_note("1", "First", "a"),
            make_note("2", "Second", "b"),
        ]);
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log: storage.event_log_mut(),
        };
        let result = (tool.handler)(&json!({}), &mut ctx).unwrap();
        assert_eq!(result["count"], 2);
        let notes = result["notes"].as_array().unwrap();
        assert_eq!(notes[0]["title"], "Second");
    }

    #[test]
    fn latest_returns_most_recent() {
        let tool = latest_tool();
        let mut ws = setup_workspace_with_notes(vec![
            make_note("1", "Old", "old content"),
            make_note("2", "New", "new content"),
        ]);
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log: storage.event_log_mut(),
        };
        let result = (tool.handler)(&json!({}), &mut ctx).unwrap();
        assert_eq!(result["found"], true);
        assert_eq!(result["note"]["title"], "New");
        assert_eq!(result["note"]["content"], "new content");
    }

    #[test]
    fn latest_returns_not_found_when_empty() {
        let tool = latest_tool();
        let mut ws = Workspace::new("test".to_string());
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log: storage.event_log_mut(),
        };
        let result = (tool.handler)(&json!({}), &mut ctx).unwrap();
        assert_eq!(result["found"], false);
    }

    #[test]
    fn create_adds_note() {
        let tool = create_tool();
        let mut ws = Workspace::new("test".to_string());
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log: storage.event_log_mut(),
        };
        let args = json!({"title": "My Note", "content": "Hello world"});
        let result = (tool.handler)(&args, &mut ctx).unwrap();
        assert_eq!(result["note"]["title"], "My Note");
        assert_eq!(result["note"]["content"], "Hello world");
        assert!(result["id"].as_str().is_some());

        // Verify note was stored
        let notes = get_notes(ctx.workspace);
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn update_modifies_existing_note() {
        let tool = update_tool();
        let mut ws = setup_workspace_with_notes(vec![
            make_note("note-1", "Original", "original content"),
        ]);
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log: storage.event_log_mut(),
        };
        let args = json!({"id": "note-1", "title": "Updated", "content": "new content"});
        let result = (tool.handler)(&args, &mut ctx).unwrap();
        assert_eq!(result["found"], true);
        assert_eq!(result["note"]["title"], "Updated");
        assert_eq!(result["note"]["content"], "new content");
    }

    #[test]
    fn update_returns_not_found_for_missing_id() {
        let tool = update_tool();
        let mut ws = setup_workspace_with_notes(vec![
            make_note("note-1", "Exists", "content"),
        ]);
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log: storage.event_log_mut(),
        };
        let args = json!({"id": "nonexistent"});
        let result = (tool.handler)(&args, &mut ctx).unwrap();
        assert_eq!(result["found"], false);
    }

    #[test]
    fn delete_removes_note() {
        let tool = delete_tool();
        let mut ws = setup_workspace_with_notes(vec![
            make_note("note-1", "To Delete", "content"),
            make_note("note-2", "To Keep", "content"),
        ]);
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log: storage.event_log_mut(),
        };
        let args = json!({"id": "note-1"});
        let result = (tool.handler)(&args, &mut ctx).unwrap();
        assert_eq!(result["deleted"], true);

        let notes = get_notes(ctx.workspace);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0]["id"], "note-2");
    }

    #[test]
    fn delete_returns_false_for_missing_id() {
        let tool = delete_tool();
        let mut ws = setup_workspace_with_notes(vec![
            make_note("note-1", "Exists", "content"),
        ]);
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log: storage.event_log_mut(),
        };
        let args = json!({"id": "nonexistent"});
        let result = (tool.handler)(&args, &mut ctx).unwrap();
        assert_eq!(result["deleted"], false);
    }

    #[test]
    fn delete_tool_is_destructive() {
        let tool = delete_tool();
        assert_eq!(tool.risk_level, RiskLevel::Destructive);
    }

    #[test]
    fn routing_metadata_has_correct_app_id() {
        let meta = routing_metadata();
        assert_eq!(meta.app_id, "notes");
        assert!(meta.keywords.contains(&"note".to_string()));
        assert!(meta.verbs.contains(&"delete".to_string()));
    }
}
