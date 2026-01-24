use std::collections::HashMap;

use crate::error::{CoreError, CoreResult};
use super::invariants::validate_invariants;
use super::state::{ApplicationInstance, ApplicationStatus, InstanceId, Workspace};

fn now_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Execute a fallible operation on the workspace with automatic rollback on error.
fn with_rollback<F, T>(workspace: &mut Workspace, f: F) -> CoreResult<T>
where
    F: FnOnce(&mut Workspace) -> CoreResult<T>,
{
    let snapshot = workspace.clone();
    match f(workspace) {
        Ok(val) => Ok(val),
        Err(e) => {
            *workspace = snapshot;
            Err(e)
        }
    }
}

/// Open a new application instance in the workspace.
///
/// If `dedupe_key` is provided and an instance with the same `app_id` and matching
/// dedupe context already exists, returns the existing instance ID (idempotent).
pub fn open_application(
    workspace: &mut Workspace,
    app_id: &str,
    dedupe_key: Option<&str>,
) -> CoreResult<InstanceId> {
    // Dedupe: if a key is provided, check for an existing active instance with same app_id
    // that has a matching "dedupe_key" in its context.
    if let Some(key) = dedupe_key {
        for instance in workspace.instances.values() {
            if instance.app_id == app_id && instance.status == ApplicationStatus::Active {
                if let Some(serde_json::Value::String(existing_key)) =
                    instance.context.get("dedupe_key")
                {
                    if existing_key == key {
                        return Ok(instance.instance_id.clone());
                    }
                }
            }
        }
    }

    let app_id = app_id.to_string();
    let dedupe_key = dedupe_key.map(|s| s.to_string());

    with_rollback(workspace, move |ws| {
        let instance_id = Workspace::new_instance_id();
        let mut context = HashMap::new();
        if let Some(key) = &dedupe_key {
            context.insert(
                "dedupe_key".to_string(),
                serde_json::Value::String(key.clone()),
            );
        }

        let instance = ApplicationInstance {
            instance_id: instance_id.clone(),
            app_id,
            status: ApplicationStatus::Active,
            context,
            mounted_tools: vec![],
        };

        ws.instances.insert(instance_id.clone(), instance);
        ws.last_modified = now_timestamp();

        validate_invariants(ws)?;
        Ok(instance_id)
    })
}

/// Close an application instance. Idempotent — closing a missing instance is a no-op.
///
/// Clears focus if it pointed to this instance.
pub fn close_application(workspace: &mut Workspace, instance_id: &str) -> CoreResult<()> {
    let instance_id = instance_id.to_string();

    with_rollback(workspace, move |ws| {
        if ws.instances.remove(&instance_id).is_some() {
            if ws.focus.as_deref() == Some(instance_id.as_str()) {
                ws.focus = None;
            }
            ws.last_modified = now_timestamp();
        }

        validate_invariants(ws)?;
        Ok(())
    })
}

/// Set focus to an application instance. The instance must exist and be active.
pub fn focus_application(workspace: &mut Workspace, instance_id: &str) -> CoreResult<()> {
    let instance_id = instance_id.to_string();

    with_rollback(workspace, move |ws| {
        let instance = ws.instances.get(&instance_id).ok_or_else(|| {
            CoreError::InvalidInput(format!("instance not found: {instance_id}"))
        })?;

        if instance.status != ApplicationStatus::Active {
            return Err(CoreError::InvalidInput(format!(
                "cannot focus inactive instance: {instance_id}"
            )));
        }

        ws.focus = Some(instance_id);
        ws.last_modified = now_timestamp();

        validate_invariants(ws)?;
        Ok(())
    })
}

/// Mount tools to an active application instance.
pub fn mount_tools(
    workspace: &mut Workspace,
    instance_id: &str,
    tool_ids: Vec<String>,
) -> CoreResult<()> {
    let instance_id = instance_id.to_string();

    with_rollback(workspace, move |ws| {
        let instance = ws.instances.get(&instance_id).ok_or_else(|| {
            CoreError::InvalidInput(format!("instance not found: {instance_id}"))
        })?;

        if instance.status != ApplicationStatus::Active {
            return Err(CoreError::InvalidInput(format!(
                "cannot mount tools on inactive instance: {instance_id}"
            )));
        }

        let instance = ws.instances.get_mut(&instance_id).unwrap();
        for tool_id in tool_ids {
            if !instance.mounted_tools.contains(&tool_id) {
                instance.mounted_tools.push(tool_id);
            }
        }
        ws.last_modified = now_timestamp();

        validate_invariants(ws)?;
        Ok(())
    })
}

/// Unmount tools from an application instance. Missing tools are silently ignored.
pub fn unmount_tools(
    workspace: &mut Workspace,
    instance_id: &str,
    tool_ids: Vec<String>,
) -> CoreResult<()> {
    let instance_id = instance_id.to_string();

    with_rollback(workspace, move |ws| {
        let instance = ws.instances.get_mut(&instance_id).ok_or_else(|| {
            CoreError::InvalidInput(format!("instance not found: {instance_id}"))
        })?;

        instance.mounted_tools.retain(|t| !tool_ids.contains(t));
        ws.last_modified = now_timestamp();

        validate_invariants(ws)?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_workspace() -> Workspace {
        Workspace::new("test".to_string())
    }

    #[test]
    fn open_application_creates_instance() {
        let mut ws = make_workspace();
        let id = open_application(&mut ws, "my-app", None).unwrap();
        assert!(ws.instances.contains_key(&id));
        assert_eq!(ws.instances[&id].app_id, "my-app");
        assert_eq!(ws.instances[&id].status, ApplicationStatus::Active);
    }

    #[test]
    fn open_application_with_dedupe_returns_existing() {
        let mut ws = make_workspace();
        let id1 = open_application(&mut ws, "my-app", Some("key-1")).unwrap();
        let id2 = open_application(&mut ws, "my-app", Some("key-1")).unwrap();
        assert_eq!(id1, id2);
        assert_eq!(ws.instances.len(), 1);
    }

    #[test]
    fn open_application_different_dedupe_creates_new() {
        let mut ws = make_workspace();
        let id1 = open_application(&mut ws, "my-app", Some("key-1")).unwrap();
        let id2 = open_application(&mut ws, "my-app", Some("key-2")).unwrap();
        assert_ne!(id1, id2);
        assert_eq!(ws.instances.len(), 2);
    }

    #[test]
    fn close_application_removes_instance() {
        let mut ws = make_workspace();
        let id = open_application(&mut ws, "my-app", None).unwrap();
        ws.focus = Some(id.clone());
        close_application(&mut ws, &id).unwrap();
        assert!(!ws.instances.contains_key(&id));
        assert_eq!(ws.focus, None);
    }

    #[test]
    fn close_application_missing_is_noop() {
        let mut ws = make_workspace();
        assert!(close_application(&mut ws, "nonexistent").is_ok());
    }

    #[test]
    fn focus_application_sets_focus() {
        let mut ws = make_workspace();
        let id = open_application(&mut ws, "my-app", None).unwrap();
        focus_application(&mut ws, &id).unwrap();
        assert_eq!(ws.focus, Some(id));
    }

    #[test]
    fn focus_application_missing_errors() {
        let mut ws = make_workspace();
        let err = focus_application(&mut ws, "missing").unwrap_err();
        match err {
            CoreError::InvalidInput(msg) => assert!(msg.contains("not found")),
            _ => panic!("expected InvalidInput"),
        }
    }

    #[test]
    fn focus_application_inactive_errors() {
        let mut ws = make_workspace();
        let id = open_application(&mut ws, "my-app", None).unwrap();
        ws.instances.get_mut(&id).unwrap().status = ApplicationStatus::Inactive;
        let err = focus_application(&mut ws, &id).unwrap_err();
        match err {
            CoreError::InvalidInput(msg) => assert!(msg.contains("inactive")),
            _ => panic!("expected InvalidInput"),
        }
    }

    #[test]
    fn mount_tools_adds_tools() {
        let mut ws = make_workspace();
        let id = open_application(&mut ws, "my-app", None).unwrap();
        mount_tools(&mut ws, &id, vec!["t1".to_string(), "t2".to_string()]).unwrap();
        let inst = &ws.instances[&id];
        assert_eq!(inst.mounted_tools, vec!["t1", "t2"]);
    }

    #[test]
    fn mount_tools_inactive_errors() {
        let mut ws = make_workspace();
        let id = open_application(&mut ws, "my-app", None).unwrap();
        ws.instances.get_mut(&id).unwrap().status = ApplicationStatus::Inactive;
        let err = mount_tools(&mut ws, &id, vec!["t1".to_string()]).unwrap_err();
        match err {
            CoreError::InvalidInput(msg) => assert!(msg.contains("inactive")),
            _ => panic!("expected InvalidInput"),
        }
    }

    #[test]
    fn mount_tools_no_duplicates() {
        let mut ws = make_workspace();
        let id = open_application(&mut ws, "my-app", None).unwrap();
        mount_tools(&mut ws, &id, vec!["t1".to_string()]).unwrap();
        mount_tools(&mut ws, &id, vec!["t1".to_string(), "t2".to_string()]).unwrap();
        let inst = &ws.instances[&id];
        assert_eq!(inst.mounted_tools, vec!["t1", "t2"]);
    }

    #[test]
    fn unmount_tools_removes_specified() {
        let mut ws = make_workspace();
        let id = open_application(&mut ws, "my-app", None).unwrap();
        mount_tools(&mut ws, &id, vec!["t1".to_string(), "t2".to_string(), "t3".to_string()])
            .unwrap();
        unmount_tools(&mut ws, &id, vec!["t2".to_string()]).unwrap();
        let inst = &ws.instances[&id];
        assert_eq!(inst.mounted_tools, vec!["t1", "t3"]);
    }

    #[test]
    fn unmount_tools_missing_is_noop() {
        let mut ws = make_workspace();
        let id = open_application(&mut ws, "my-app", None).unwrap();
        mount_tools(&mut ws, &id, vec!["t1".to_string()]).unwrap();
        unmount_tools(&mut ws, &id, vec!["nonexistent".to_string()]).unwrap();
        let inst = &ws.instances[&id];
        assert_eq!(inst.mounted_tools, vec!["t1"]);
    }
}
