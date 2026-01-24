//! Workspace rehydration by replaying events.

use super::event::Event;
use crate::error::CoreResult;
use crate::workspace::{apply_patch, Workspace};

/// Replay workspace-related events to rebuild a `Workspace` from scratch.
///
/// Starts with a default workspace and applies each `WorkspacePatched` event
/// in order. Non-workspace events are skipped.
pub fn replay_workspace(events: &[Event]) -> CoreResult<Workspace> {
    let mut workspace = Workspace::new("replayed".to_string());

    for event in events {
        if let Event::WorkspacePatched { patch, .. } = event {
            apply_patch(&mut workspace, patch.clone())?;
        }
    }

    Ok(workspace)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{WorkspaceOp, WorkspacePatch};
    use std::time::SystemTime;
    use uuid::Uuid;

    fn make_workspace_patched(patch: WorkspacePatch) -> Event {
        Event::WorkspacePatched {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            patch,
            workspace_hash_before: "before".to_string(),
            workspace_hash_after: "after".to_string(),
        }
    }

    #[test]
    fn empty_events_yields_default_workspace() {
        let ws = replay_workspace(&[]).unwrap();
        assert!(ws.instances.is_empty());
        assert_eq!(ws.focus, None);
        assert_eq!(ws.session_id, "replayed");
    }

    #[test]
    fn replay_open_application() {
        let events = vec![make_workspace_patched(WorkspacePatch {
            operations: vec![WorkspaceOp::OpenApplication {
                app_id: "app-1".to_string(),
                dedupe_key: None,
            }],
        })];

        let ws = replay_workspace(&events).unwrap();
        assert_eq!(ws.instances.len(), 1);
    }

    #[test]
    fn replay_skips_non_workspace_events() {
        let events = vec![
            Event::UserMessage {
                id: Uuid::new_v4(),
                timestamp: SystemTime::now(),
                text: "ignored".to_string(),
            },
            make_workspace_patched(WorkspacePatch {
                operations: vec![WorkspaceOp::OpenApplication {
                    app_id: "app-2".to_string(),
                    dedupe_key: None,
                }],
            }),
            Event::ErrorRaised {
                id: Uuid::new_v4(),
                timestamp: SystemTime::now(),
                code: "E1".to_string(),
                message: "also ignored".to_string(),
            },
        ];

        let ws = replay_workspace(&events).unwrap();
        assert_eq!(ws.instances.len(), 1);
    }

    #[test]
    fn replay_multiple_patches_in_order() {
        let events = vec![
            make_workspace_patched(WorkspacePatch {
                operations: vec![WorkspaceOp::OpenApplication {
                    app_id: "app-a".to_string(),
                    dedupe_key: Some("key-a".to_string()),
                }],
            }),
            make_workspace_patched(WorkspacePatch {
                operations: vec![WorkspaceOp::OpenApplication {
                    app_id: "app-b".to_string(),
                    dedupe_key: None,
                }],
            }),
        ];

        let ws = replay_workspace(&events).unwrap();
        assert_eq!(ws.instances.len(), 2);
    }

    #[test]
    fn replay_patch_failure_propagates_error() {
        // FocusApplication on a nonexistent instance returns an error.
        let events = vec![make_workspace_patched(WorkspacePatch {
            operations: vec![WorkspaceOp::FocusApplication {
                instance_id: "nonexistent".to_string(),
            }],
        })];

        let result = replay_workspace(&events);
        assert!(result.is_err());
    }
}
