//! Clipboard built-in app: list and latest tools.

use std::sync::Arc;

use serde_json::json;

use crate::platform::ClipboardProvider;
use crate::routing::RoutingMetadata;
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
        handler: Box::new(move |args, _ctx| {
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(50) as usize;

            let entries = provider.get_history();
            let limited: Vec<_> = entries.iter().rev().take(limit).cloned().collect();
            let count = limited.len();

            Ok(json!({
                "entries": limited,
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
        handler: Box::new(move |_args, _ctx| {
            match provider.get_latest() {
                Some(entry) => Ok(json!({
                    "entry": entry,
                    "found": true
                })),
                None => Ok(json!({
                    "entry": null,
                    "found": false
                })),
            }
        }),
    }
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
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log: storage.event_log_mut(),
        };
        let result = (tool.handler)(&json!({}), &mut ctx).unwrap();
        assert_eq!(result["count"], 0);
        assert!(result["entries"].as_array().unwrap().is_empty());
    }

    #[test]
    fn list_returns_entries_in_reverse_order() {
        let provider = Arc::new(MockClipboardProvider::new(vec![
            json!({"text": "first"}),
            json!({"text": "second"}),
            json!({"text": "third"}),
        ]));
        let tool = list_tool(provider);
        let mut ws = Workspace::new("test".to_string());
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log: storage.event_log_mut(),
        };
        let result = (tool.handler)(&json!({}), &mut ctx).unwrap();
        assert_eq!(result["count"], 3);
        let entries = result["entries"].as_array().unwrap();
        assert_eq!(entries[0]["text"], "third");
        assert_eq!(entries[2]["text"], "first");
    }

    #[test]
    fn list_respects_limit() {
        let provider = Arc::new(MockClipboardProvider::new(vec![
            json!({"text": "a"}),
            json!({"text": "b"}),
            json!({"text": "c"}),
        ]));
        let tool = list_tool(provider);
        let mut ws = Workspace::new("test".to_string());
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log: storage.event_log_mut(),
        };
        let result = (tool.handler)(&json!({"limit": 2}), &mut ctx).unwrap();
        assert_eq!(result["count"], 2);
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
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log: storage.event_log_mut(),
        };
        let result = (tool.handler)(&json!({}), &mut ctx).unwrap();
        assert_eq!(result["found"], true);
        assert_eq!(result["entry"]["text"], "newest");
    }

    #[test]
    fn latest_returns_not_found_when_empty() {
        let provider = Arc::new(MockClipboardProvider::new(vec![]));
        let tool = latest_tool(provider);
        let mut ws = Workspace::new("test".to_string());
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log: storage.event_log_mut(),
        };
        let result = (tool.handler)(&json!({}), &mut ctx).unwrap();
        assert_eq!(result["found"], false);
        assert!(result["entry"].is_null());
    }

    #[test]
    fn routing_metadata_has_correct_app_id() {
        let meta = routing_metadata();
        assert_eq!(meta.app_id, "clipboard");
        assert!(meta.keywords.contains(&"clipboard".to_string()));
    }
}
