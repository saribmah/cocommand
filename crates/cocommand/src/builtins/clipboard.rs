//! Clipboard built-in app: list and latest tools.

use std::sync::Arc;
use std::time::SystemTime;

use serde_json::json;
use uuid::Uuid;

use crate::platform::ClipboardProvider;
use crate::routing::RoutingMetadata;
use crate::storage::ClipboardEntry;
use crate::tools::schema::{RiskLevel, ToolDefinition};
use crate::tools::registry::ToolRegistry;
use crate::routing::Router;

/// App identifier.
pub const APP_ID: &str = "clipboard";

/// Register clipboard tools and routing metadata.
pub fn register(registry: &mut ToolRegistry, router: &mut Router, provider: Arc<dyn ClipboardProvider>) {
    registry.register_kernel_tool(list_tool(Arc::clone(&provider)));
    registry.register_kernel_tool(latest_tool(provider));
    router.register(routing_metadata());
}

/// Routing metadata for the clipboard app.
fn routing_metadata() -> RoutingMetadata {
    RoutingMetadata {
        app_id: APP_ID.to_string(),
        keywords: vec![
            "clipboard".into(),
            "copy".into(),
            "paste".into(),
            "copied".into(),
        ],
        examples: vec![
            "show clipboard history".into(),
            "what did I copy last".into(),
            "show my clipboard".into(),
            "paste from clipboard".into(),
        ],
        verbs: vec![
            "show".into(),
            "list".into(),
            "get".into(),
            "paste".into(),
        ],
        objects: vec![
            "clipboard".into(),
            "history".into(),
            "copied".into(),
        ],
    }
}

/// Tool definition for `clipboard.list`.
fn list_tool(provider: Arc<dyn ClipboardProvider>) -> ToolDefinition {
    ToolDefinition {
        tool_id: "clipboard.list".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "limit": {"type": "integer"}
            }
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "entries": {"type": "array"},
                "count": {"type": "integer"}
            }
        }),
        risk_level: RiskLevel::Safe,
        is_kernel: false,
        handler: Box::new(move |args, ctx| {
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(50) as usize;

            // Capture-on-use: push current clipboard content into the store.
            if let Some(entry) = provider.get_latest() {
                if let Some(text) = extract_text(&entry) {
                    ctx.clipboard_store.push(ClipboardEntry {
                        id: Uuid::new_v4(),
                        content: text,
                        copied_at: SystemTime::now(),
                    });
                }
            }

            // Read history from the store (most-recent-first).
            let entries: Vec<_> = ctx.clipboard_store.list(limit)
                .into_iter()
                .map(|e| json!({
                    "id": e.id.to_string(),
                    "content": e.content,
                    "copied_at": e.copied_at
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                }))
                .collect();
            let count = entries.len();

            Ok(json!({
                "entries": entries,
                "count": count
            }))
        }),
    }
}

/// Tool definition for `clipboard.latest`.
fn latest_tool(provider: Arc<dyn ClipboardProvider>) -> ToolDefinition {
    ToolDefinition {
        tool_id: "clipboard.latest".to_string(),
        input_schema: json!({
            "type": "object"
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "entry": {},
                "found": {"type": "boolean"}
            }
        }),
        risk_level: RiskLevel::Safe,
        is_kernel: false,
        handler: Box::new(move |_args, ctx| {
            match provider.get_latest() {
                Some(entry) => {
                    // Capture-on-use: push into the clipboard store.
                    if let Some(text) = extract_text(&entry) {
                        ctx.clipboard_store.push(ClipboardEntry {
                            id: Uuid::new_v4(),
                            content: text,
                            copied_at: SystemTime::now(),
                        });
                    }
                    Ok(json!({
                        "entry": entry,
                        "found": true
                    }))
                }
                None => Ok(json!({
                    "entry": null,
                    "found": false
                })),
            }
        }),
    }
}

/// Extract text content from a clipboard provider entry.
/// Handles both plain string values and objects with a "text" field.
fn extract_text(value: &serde_json::Value) -> Option<String> {
    if let Some(s) = value.as_str() {
        return Some(s.to_string());
    }
    if let Some(text) = value.get("text").and_then(|t| t.as_str()) {
        return Some(text.to_string());
    }
    if let Some(content) = value.get("content").and_then(|t| t.as_str()) {
        return Some(content.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{MemoryStorage, Storage};
    use crate::platform::MockClipboardProvider;
    use crate::tools::schema::ExecutionContext;
    use crate::workspace::Workspace;

    #[test]
    fn list_returns_empty_when_no_history() {
        let provider = Arc::new(MockClipboardProvider::new(vec![]));
        let tool = list_tool(provider);
        let mut ws = Workspace::new("test".to_string());
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log,
            clipboard_store,
        };
        let result = (tool.handler)(&json!({}), &mut ctx).unwrap();
        assert_eq!(result["count"], 0);
        assert!(result["entries"].as_array().unwrap().is_empty());
    }

    #[test]
    fn list_captures_current_clipboard_on_use() {
        let provider = Arc::new(MockClipboardProvider::new(vec![
            json!({"text": "current"}),
        ]));
        let tool = list_tool(provider);
        let mut ws = Workspace::new("test".to_string());
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log,
            clipboard_store,
        };
        let result = (tool.handler)(&json!({}), &mut ctx).unwrap();
        assert_eq!(result["count"], 1);
        let entries = result["entries"].as_array().unwrap();
        assert_eq!(entries[0]["content"], "current");
    }

    #[test]
    fn list_returns_history_most_recent_first() {
        let provider = Arc::new(MockClipboardProvider::new(vec![
            json!({"text": "latest"}),
        ]));
        let tool = list_tool(provider);
        let mut ws = Workspace::new("test".to_string());
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());

        // Pre-populate the store with older entries.
        storage.clipboard_mut().push(ClipboardEntry {
            id: Uuid::new_v4(),
            content: "first".to_string(),
            copied_at: SystemTime::now(),
        });
        storage.clipboard_mut().push(ClipboardEntry {
            id: Uuid::new_v4(),
            content: "second".to_string(),
            copied_at: SystemTime::now(),
        });

        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log,
            clipboard_store,
        };
        let result = (tool.handler)(&json!({}), &mut ctx).unwrap();
        assert_eq!(result["count"], 3);
        let entries = result["entries"].as_array().unwrap();
        // Most recent first: "latest" (captured), "second", "first"
        assert_eq!(entries[0]["content"], "latest");
        assert_eq!(entries[1]["content"], "second");
        assert_eq!(entries[2]["content"], "first");
    }

    #[test]
    fn list_respects_limit() {
        let provider = Arc::new(MockClipboardProvider::new(vec![
            json!({"text": "current"}),
        ]));
        let tool = list_tool(provider);
        let mut ws = Workspace::new("test".to_string());
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());

        // Pre-populate with entries.
        for i in 0..5 {
            storage.clipboard_mut().push(ClipboardEntry {
                id: Uuid::new_v4(),
                content: format!("item-{i}"),
                copied_at: SystemTime::now(),
            });
        }

        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log,
            clipboard_store,
        };
        // Limit to 2 — should return 2 most recent entries.
        let result = (tool.handler)(&json!({"limit": 2}), &mut ctx).unwrap();
        assert_eq!(result["count"], 2);
    }

    #[test]
    fn list_deduplicates_consecutive() {
        let provider: Arc<dyn ClipboardProvider> = Arc::new(MockClipboardProvider::new(vec![
            json!({"text": "same"}),
        ]));
        let tool = list_tool(Arc::clone(&provider));
        let mut ws = Workspace::new("test".to_string());
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());

        // Push the same content that the provider will return.
        storage.clipboard_mut().push(ClipboardEntry {
            id: Uuid::new_v4(),
            content: "same".to_string(),
            copied_at: SystemTime::now(),
        });

        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log,
            clipboard_store,
        };
        let result = (tool.handler)(&json!({}), &mut ctx).unwrap();
        // Should not duplicate: "same" was already the last entry.
        assert_eq!(result["count"], 1);
    }

    #[test]
    fn latest_returns_most_recent_entry() {
        let provider = Arc::new(MockClipboardProvider::new(vec![
            json!({"text": "old"}),
            json!({"text": "newest"}),
        ]));
        let tool = latest_tool(provider);
        let mut ws = Workspace::new("test".to_string());
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log,
            clipboard_store,
        };
        let result = (tool.handler)(&json!({}), &mut ctx).unwrap();
        assert_eq!(result["found"], true);
        assert_eq!(result["entry"]["text"], "newest");
    }

    #[test]
    fn latest_captures_entry_into_store() {
        let provider = Arc::new(MockClipboardProvider::new(vec![
            json!({"text": "captured"}),
        ]));
        let tool = latest_tool(provider);
        let mut ws = Workspace::new("test".to_string());
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log,
            clipboard_store,
        };
        let _result = (tool.handler)(&json!({}), &mut ctx).unwrap();

        // Verify the entry was captured in the store.
        assert_eq!(ctx.clipboard_store.len(), 1);
        assert_eq!(ctx.clipboard_store.latest().unwrap().content, "captured");
    }

    #[test]
    fn latest_returns_not_found_when_empty() {
        let provider = Arc::new(MockClipboardProvider::new(vec![]));
        let tool = latest_tool(provider);
        let mut ws = Workspace::new("test".to_string());
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log,
            clipboard_store,
        };
        let result = (tool.handler)(&json!({}), &mut ctx).unwrap();
        assert_eq!(result["found"], false);
        assert!(result["entry"].is_null());
    }

    #[test]
    fn extract_text_from_string_value() {
        let val = json!("hello");
        assert_eq!(extract_text(&val), Some("hello".to_string()));
    }

    #[test]
    fn extract_text_from_object_with_text_field() {
        let val = json!({"text": "world"});
        assert_eq!(extract_text(&val), Some("world".to_string()));
    }

    #[test]
    fn extract_text_from_object_with_content_field() {
        let val = json!({"content": "data"});
        assert_eq!(extract_text(&val), Some("data".to_string()));
    }

    #[test]
    fn extract_text_returns_none_for_non_text() {
        let val = json!(42);
        assert_eq!(extract_text(&val), None);
    }

    #[test]
    fn routing_metadata_has_correct_app_id() {
        let meta = routing_metadata();
        assert_eq!(meta.app_id, "clipboard");
        assert!(meta.keywords.contains(&"clipboard".to_string()));
    }
}
