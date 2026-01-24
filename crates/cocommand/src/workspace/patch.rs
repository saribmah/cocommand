use serde::{Deserialize, Serialize};

use crate::error::CoreResult;
use super::kernel_tools;
use super::state::Workspace;

/// A single workspace operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkspaceOp {
    OpenApplication {
        app_id: String,
        dedupe_key: Option<String>,
    },
    CloseApplication {
        instance_id: String,
    },
    FocusApplication {
        instance_id: String,
    },
    MountTools {
        instance_id: String,
        tool_ids: Vec<String>,
    },
    UnmountTools {
        instance_id: String,
        tool_ids: Vec<String>,
    },
}

/// A batch of workspace operations to apply atomically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacePatch {
    pub operations: Vec<WorkspaceOp>,
}

/// Result of applying a patch.
#[derive(Debug, Clone)]
pub struct PatchResult {
    pub applied_ops: usize,
}

/// Apply a patch atomically. If any operation fails, the workspace is rolled back
/// to its state before the patch was applied.
pub fn apply_patch(workspace: &mut Workspace, patch: WorkspacePatch) -> CoreResult<PatchResult> {
    let snapshot = workspace.clone();
    let mut applied = 0;

    for op in patch.operations {
        let result = apply_op(workspace, op);
        if let Err(e) = result {
            *workspace = snapshot;
            return Err(e);
        }
        applied += 1;
    }

    Ok(PatchResult {
        applied_ops: applied,
    })
}

fn apply_op(workspace: &mut Workspace, op: WorkspaceOp) -> CoreResult<()> {
    match op {
        WorkspaceOp::OpenApplication { app_id, dedupe_key } => {
            kernel_tools::open_application(
                workspace,
                &app_id,
                dedupe_key.as_deref(),
            )?;
            Ok(())
        }
        WorkspaceOp::CloseApplication { instance_id } => {
            kernel_tools::close_application(workspace, &instance_id)
        }
        WorkspaceOp::FocusApplication { instance_id } => {
            kernel_tools::focus_application(workspace, &instance_id)
        }
        WorkspaceOp::MountTools {
            instance_id,
            tool_ids,
        } => kernel_tools::mount_tools(workspace, &instance_id, tool_ids),
        WorkspaceOp::UnmountTools {
            instance_id,
            tool_ids,
        } => kernel_tools::unmount_tools(workspace, &instance_id, tool_ids),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::kernel_tools::open_application;

    fn make_workspace() -> Workspace {
        Workspace::new("test".to_string())
    }

    #[test]
    fn single_op_patch_applies() {
        let mut ws = make_workspace();
        let patch = WorkspacePatch {
            operations: vec![WorkspaceOp::OpenApplication {
                app_id: "app-1".to_string(),
                dedupe_key: None,
            }],
        };
        let result = apply_patch(&mut ws, patch).unwrap();
        assert_eq!(result.applied_ops, 1);
        assert_eq!(ws.instances.len(), 1);
    }

    #[test]
    fn multi_op_patch_applies_atomically() {
        let mut ws = make_workspace();
        let id = open_application(&mut ws, "app-1", None).unwrap();

        let patch = WorkspacePatch {
            operations: vec![
                WorkspaceOp::MountTools {
                    instance_id: id.clone(),
                    tool_ids: vec!["t1".to_string()],
                },
                WorkspaceOp::FocusApplication {
                    instance_id: id.clone(),
                },
            ],
        };
        let result = apply_patch(&mut ws, patch).unwrap();
        assert_eq!(result.applied_ops, 2);
        assert_eq!(ws.focus, Some(id.clone()));
        assert_eq!(ws.instances[&id].mounted_tools, vec!["t1"]);
    }

    #[test]
    fn failing_op_rolls_back_all_changes() {
        let mut ws = make_workspace();
        let id = open_application(&mut ws, "app-1", None).unwrap();
        let original_instances_len = ws.instances.len();

        let patch = WorkspacePatch {
            operations: vec![
                // This succeeds
                WorkspaceOp::MountTools {
                    instance_id: id.clone(),
                    tool_ids: vec!["t1".to_string()],
                },
                // This fails — instance doesn't exist
                WorkspaceOp::FocusApplication {
                    instance_id: "nonexistent".to_string(),
                },
            ],
        };
        let err = apply_patch(&mut ws, patch);
        assert!(err.is_err());

        // Workspace should be rolled back — no tools mounted
        assert_eq!(ws.instances.len(), original_instances_len);
        assert!(ws.instances[&id].mounted_tools.is_empty());
        assert_eq!(ws.focus, None);
    }

    #[test]
    fn empty_patch_is_noop() {
        let mut ws = make_workspace();
        let patch = WorkspacePatch {
            operations: vec![],
        };
        let result = apply_patch(&mut ws, patch).unwrap();
        assert_eq!(result.applied_ops, 0);
    }
}
