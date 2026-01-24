//! Built-in app implementations (Core-8).
//!
//! Provides Clipboard, Notes, and Calculator as always-available built-in applications.
//! Each registers its tools and routing metadata at startup via [`register_builtins`].

pub mod calculator;
pub mod clipboard;
pub mod notes;

use std::sync::Arc;

use crate::platform::{ClipboardProvider, NullClipboardProvider};
use crate::routing::Router;
use crate::tools::registry::ToolRegistry;

/// Register all built-in app tools and routing metadata.
///
/// Call this during core startup to make built-in apps available for
/// routing and execution. Uses [`NullClipboardProvider`] by default;
/// call [`register_builtins_with`] to supply a real provider.
pub fn register_builtins(registry: &mut ToolRegistry, router: &mut Router) {
    register_builtins_with(registry, router, Arc::new(NullClipboardProvider));
}

/// Register all built-in apps with an explicit clipboard provider.
pub fn register_builtins_with(
    registry: &mut ToolRegistry,
    router: &mut Router,
    clipboard_provider: Arc<dyn ClipboardProvider>,
) {
    clipboard::register(registry, router, clipboard_provider);
    notes::register(registry, router);
    calculator::register(registry, router);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::ParsedCommand;
    use crate::permissions::PermissionStore;
    use crate::platform::MockClipboardProvider;
    use crate::storage::{MemoryStorage, Storage};
    use crate::tools::executor::{execute_tool, ToolExecutionOutcome};
    use crate::workspace::Workspace;
    use serde_json::json;
    use uuid::Uuid;

    fn setup() -> (ToolRegistry, Router, Workspace, Box<dyn Storage>, PermissionStore) {
        setup_with_clipboard(Arc::new(MockClipboardProvider::new(vec![])))
    }

    fn setup_with_clipboard(
        provider: Arc<dyn ClipboardProvider>,
    ) -> (ToolRegistry, Router, Workspace, Box<dyn Storage>, PermissionStore) {
        let mut registry = ToolRegistry::new();
        let mut router = Router::new();
        register_builtins_with(&mut registry, &mut router, provider);
        let workspace = Workspace::new("test-session".to_string());
        let storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let permission_store = PermissionStore::new();
        (registry, router, workspace, storage, permission_store)
    }

    // --- End-to-end: route → execute ---

    #[test]
    fn end_to_end_show_last_note() {
        let (registry, router, mut workspace, mut storage, permission_store) = setup();

        // Seed a note in workspace (keyed by APP_ID)
        let note = json!({
            "id": "note-abc",
            "title": "My Note",
            "content": "hello world",
            "created_at": 1000
        });
        let mut context = std::collections::HashMap::new();
        context.insert("notes".to_string(), json!([note]));
        let instance = crate::workspace::ApplicationInstance {
            instance_id: notes::APP_ID.to_string(),
            app_id: notes::APP_ID.to_string(),
            status: crate::workspace::ApplicationStatus::Active,
            context,
            mounted_tools: vec![],
        };
        workspace.instances.insert(notes::APP_ID.to_string(), instance);

        // Route the command
        let cmd = ParsedCommand {
            raw_text: "show last note".to_string(),
            normalized_text: "show last note".to_string(),
            tags: vec![],
        };
        let routing_result = router.route(&cmd);
        assert!(!routing_result.candidates.is_empty());
        assert_eq!(routing_result.candidates[0].app_id, "notes");

        // Execute notes.latest (kernel tool, instance_id doesn't matter for lookup)
        let result = execute_tool(
            &registry,
            &mut workspace,
            storage.event_log_mut(),
            &permission_store,
            notes::APP_ID,
            "notes.latest",
            json!({}),
            Uuid::new_v4(),
        );

        match result {
            ToolExecutionOutcome::Executed(exec) => {
                assert_eq!(exec.result["found"], true);
                assert_eq!(exec.result["note"]["title"], "My Note");
                assert_eq!(exec.result["note"]["content"], "hello world");
            }
            other => panic!("expected Executed, got {:?}", other),
        }
    }

    #[test]
    fn end_to_end_delete_note_needs_confirmation() {
        let (registry, router, mut workspace, mut storage, permission_store) = setup();

        // Seed a note
        let note = json!({
            "id": "note-xyz",
            "title": "Delete Me",
            "content": "to be deleted",
            "created_at": 1000
        });
        let mut context = std::collections::HashMap::new();
        context.insert("notes".to_string(), json!([note]));
        let instance = crate::workspace::ApplicationInstance {
            instance_id: notes::APP_ID.to_string(),
            app_id: notes::APP_ID.to_string(),
            status: crate::workspace::ApplicationStatus::Active,
            context,
            mounted_tools: vec![],
        };
        workspace.instances.insert(notes::APP_ID.to_string(), instance);

        // Route the command
        let cmd = ParsedCommand {
            raw_text: "delete last note".to_string(),
            normalized_text: "delete last note".to_string(),
            tags: vec![],
        };
        let routing_result = router.route(&cmd);
        assert!(!routing_result.candidates.is_empty());
        assert_eq!(routing_result.candidates[0].app_id, "notes");

        // Execute notes.delete — should require confirmation (Destructive risk)
        let result = execute_tool(
            &registry,
            &mut workspace,
            storage.event_log_mut(),
            &permission_store,
            notes::APP_ID,
            "notes.delete",
            json!({"id": "note-xyz"}),
            Uuid::new_v4(),
        );

        assert!(
            matches!(result, ToolExecutionOutcome::NeedsConfirmation { .. }),
            "expected NeedsConfirmation for destructive delete"
        );
    }

    #[test]
    fn all_builtins_register_routing_metadata() {
        let (_registry, router, _workspace, _storage, _permission_store) = setup();

        let cmd = ParsedCommand {
            raw_text: "calculate 2+2".to_string(),
            normalized_text: "calculate 2+2".to_string(),
            tags: vec![],
        };
        let result = router.route(&cmd);
        assert!(result.candidates.iter().any(|c| c.app_id == "calculator"));

        let cmd = ParsedCommand {
            raw_text: "show clipboard".to_string(),
            normalized_text: "show clipboard".to_string(),
            tags: vec![],
        };
        let result = router.route(&cmd);
        assert!(result.candidates.iter().any(|c| c.app_id == "clipboard"));

        let cmd = ParsedCommand {
            raw_text: "create a note".to_string(),
            normalized_text: "create a note".to_string(),
            tags: vec![],
        };
        let result = router.route(&cmd);
        assert!(result.candidates.iter().any(|c| c.app_id == "notes"));
    }

    #[test]
    fn calculator_eval_end_to_end() {
        let (registry, _router, mut workspace, mut storage, permission_store) = setup();

        // Kernel tools are accessible from any instance_id
        let result = execute_tool(
            &registry,
            &mut workspace,
            storage.event_log_mut(),
            &permission_store,
            "any-instance",
            "calculator.eval",
            json!({"expression": "3 * (4 + 5)"}),
            Uuid::new_v4(),
        );

        match result {
            ToolExecutionOutcome::Executed(exec) => {
                assert_eq!(exec.result["result"], 27.0);
            }
            other => panic!("expected Executed, got {:?}", other),
        }
    }

    #[test]
    fn clipboard_latest_end_to_end() {
        let provider = Arc::new(MockClipboardProvider::new(vec![
            json!({"text": "old copy"}),
            json!({"text": "latest copy"}),
        ]));
        let (registry, _router, mut workspace, mut storage, permission_store) =
            setup_with_clipboard(provider);

        let result = execute_tool(
            &registry,
            &mut workspace,
            storage.event_log_mut(),
            &permission_store,
            "any-instance",
            "clipboard.latest",
            json!({}),
            Uuid::new_v4(),
        );

        match result {
            ToolExecutionOutcome::Executed(exec) => {
                assert_eq!(exec.result["found"], true);
                assert_eq!(exec.result["entry"]["text"], "latest copy");
            }
            other => panic!("expected Executed, got {:?}", other),
        }
    }
}
