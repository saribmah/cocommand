use cocommand::storage::MemoryStorage;
use cocommand::tools::{execute_tool, ToolExecutionOutcome};
use cocommand::Core;
use serde_json::json;
use uuid::Uuid;

fn make_core() -> Core {
    let mut core = Core::new(Box::new(MemoryStorage::new()));
    core.register_builtins();
    core
}

fn execute_tool_with_core(
    core: &Core,
    instance_id: &str,
    tool_id: &str,
    args: serde_json::Value,
) -> ToolExecutionOutcome {
    let runtime = core.tool_runtime(instance_id.to_string());
    let registry = runtime.registry.lock().expect("registry lock");
    let mut workspace = runtime.workspace.lock().expect("workspace lock");
    let mut storage = runtime.storage.lock().expect("storage lock");
    let permission_store = runtime.permission_store.lock().expect("permission store lock");
    let (event_log, clipboard_store) = storage.split_event_clipboard_mut();

    execute_tool(
        &registry,
        &mut workspace,
        event_log,
        clipboard_store,
        &permission_store,
        &runtime.instance_id,
        tool_id,
        args,
        Uuid::new_v4(),
    )
}

#[test]
fn e2e_notes_create_then_list() {
    let core = make_core();

    let created = execute_tool_with_core(
        &core,
        "notes",
        "notes.create",
        json!({ "title": "Laundry Reminder", "content": "Pickup laundry" }),
    );

    let note_id = match created {
        ToolExecutionOutcome::Executed(exec) => exec.result["id"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        other => panic!("expected Executed for notes.create, got {other:?}"),
    };

    let listed = execute_tool_with_core(&core, "notes", "notes.list", json!({ "limit": 10 }));

    match listed {
        ToolExecutionOutcome::Executed(exec) => {
            assert_eq!(exec.result["count"], 1);
            assert_eq!(exec.result["notes"][0]["id"], note_id);
            assert_eq!(exec.result["notes"][0]["title"], "Laundry Reminder");
        }
        other => panic!("expected Executed for notes.list, got {other:?}"),
    }
}

#[test]
fn e2e_notes_delete_requires_confirmation() {
    let mut core = make_core();

    let created = execute_tool_with_core(
        &core,
        "notes",
        "notes.create",
        json!({ "title": "Temp", "content": "Delete me" }),
    );

    let note_id = match created {
        ToolExecutionOutcome::Executed(exec) => exec.result["id"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        other => panic!("expected Executed for notes.create, got {other:?}"),
    };

    let delete_result = execute_tool_with_core(
        &core,
        "notes",
        "notes.delete",
        json!({ "id": note_id }),
    );

    let confirmation_id = match delete_result {
        ToolExecutionOutcome::NeedsConfirmation { confirmation_id } => confirmation_id,
        other => panic!("expected NeedsConfirmation for notes.delete, got {other:?}"),
    };

    let response = core.confirm_action(&confirmation_id, false).unwrap();
    match response {
        cocommand::CoreResponse::Artifact { content, .. } => {
            assert!(content.contains("cancelled"));
        }
        other => panic!("expected Artifact confirmation response, got {other:?}"),
    }
}

#[test]
fn e2e_submit_command_preview() {
    let mut core = make_core();

    let response = core.submit_command("show last note").unwrap();
    match response {
        cocommand::CoreResponse::Preview { title, content } => {
            assert!(title.contains("notes"));
            assert!(!content.is_empty());
        }
        other => panic!("expected Preview response, got {other:?}"),
    }
}
