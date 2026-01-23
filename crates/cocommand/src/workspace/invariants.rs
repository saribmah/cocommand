use std::collections::HashSet;

use crate::error::{CoreError, CoreResult};
use super::state::{ApplicationStatus, Workspace};

/// Validate all workspace invariants. Returns an error if any invariant is violated.
pub fn validate_invariants(workspace: &Workspace) -> CoreResult<()> {
    // Invariant 1: Focus validity — if focus is set, it must reference an existing Active instance.
    if let Some(ref focus_id) = workspace.focus {
        match workspace.instances.get(focus_id) {
            None => {
                return Err(CoreError::InvariantViolation(format!(
                    "focus points to nonexistent instance: {focus_id}"
                )));
            }
            Some(instance) => {
                if instance.status != ApplicationStatus::Active {
                    return Err(CoreError::InvariantViolation(format!(
                        "focus points to inactive instance: {focus_id}"
                    )));
                }
            }
        }
    }

    // Invariant 2: Mounted tools validity — instances with mounted tools must be Active.
    for (id, instance) in &workspace.instances {
        if !instance.mounted_tools.is_empty() && instance.status != ApplicationStatus::Active {
            return Err(CoreError::InvariantViolation(format!(
                "inactive instance {id} has mounted tools"
            )));
        }
    }

    // Invariant 3: Per-instance tool uniqueness — no duplicate tool IDs within a single instance.
    for (id, instance) in &workspace.instances {
        let mut seen: HashSet<&str> = HashSet::new();
        for tool_id in &instance.mounted_tools {
            if !seen.insert(tool_id.as_str()) {
                return Err(CoreError::InvariantViolation(format!(
                    "duplicate tool {tool_id} within instance {id}"
                )));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::state::{ApplicationInstance, ApplicationStatus};
    use std::collections::HashMap;

    fn make_workspace() -> Workspace {
        Workspace::new("test".to_string())
    }

    fn active_instance(id: &str) -> ApplicationInstance {
        ApplicationInstance {
            instance_id: id.to_string(),
            app_id: "app".to_string(),
            status: ApplicationStatus::Active,
            context: HashMap::new(),
            mounted_tools: vec![],
        }
    }

    #[test]
    fn valid_workspace_passes() {
        let mut ws = make_workspace();
        let inst = active_instance("i1");
        ws.instances.insert("i1".to_string(), inst);
        ws.focus = Some("i1".to_string());
        assert!(validate_invariants(&ws).is_ok());
    }

    #[test]
    fn empty_workspace_passes() {
        let ws = make_workspace();
        assert!(validate_invariants(&ws).is_ok());
    }

    #[test]
    fn focus_nonexistent_instance_fails() {
        let mut ws = make_workspace();
        ws.focus = Some("missing".to_string());
        let err = validate_invariants(&ws).unwrap_err();
        match err {
            CoreError::InvariantViolation(msg) => {
                assert!(msg.contains("nonexistent"), "got: {msg}");
            }
            _ => panic!("expected InvariantViolation"),
        }
    }

    #[test]
    fn focus_inactive_instance_fails() {
        let mut ws = make_workspace();
        let mut inst = active_instance("i1");
        inst.status = ApplicationStatus::Inactive;
        ws.instances.insert("i1".to_string(), inst);
        ws.focus = Some("i1".to_string());
        let err = validate_invariants(&ws).unwrap_err();
        match err {
            CoreError::InvariantViolation(msg) => {
                assert!(msg.contains("inactive"), "got: {msg}");
            }
            _ => panic!("expected InvariantViolation"),
        }
    }

    #[test]
    fn mounted_tools_on_inactive_instance_fails() {
        let mut ws = make_workspace();
        let mut inst = active_instance("i1");
        inst.status = ApplicationStatus::Inactive;
        inst.mounted_tools = vec!["tool-1".to_string()];
        ws.instances.insert("i1".to_string(), inst);
        let err = validate_invariants(&ws).unwrap_err();
        match err {
            CoreError::InvariantViolation(msg) => {
                assert!(msg.contains("inactive"), "got: {msg}");
            }
            _ => panic!("expected InvariantViolation"),
        }
    }

    #[test]
    fn same_tool_across_instances_passes() {
        let mut ws = make_workspace();
        let mut inst1 = active_instance("i1");
        inst1.mounted_tools = vec!["shared-tool".to_string()];
        let mut inst2 = active_instance("i2");
        inst2.mounted_tools = vec!["shared-tool".to_string()];
        ws.instances.insert("i1".to_string(), inst1);
        ws.instances.insert("i2".to_string(), inst2);
        assert!(validate_invariants(&ws).is_ok());
    }

    #[test]
    fn duplicate_tool_within_instance_fails() {
        let mut ws = make_workspace();
        let mut inst = active_instance("i1");
        inst.mounted_tools = vec!["tool-a".to_string(), "tool-a".to_string()];
        ws.instances.insert("i1".to_string(), inst);
        let err = validate_invariants(&ws).unwrap_err();
        match err {
            CoreError::InvariantViolation(msg) => {
                assert!(msg.contains("duplicate tool tool-a"), "got: {msg}");
            }
            _ => panic!("expected InvariantViolation"),
        }
    }
}
