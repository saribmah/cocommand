use crate::error::{CoreError, CoreResult};
use crate::types::{ActionSummary, ConfirmationDecision, CoreResponse};
use crate::workspace::Workspace;

/// Primary facade for the cocommand engine.
///
/// All orchestration flows are accessed through this struct.
pub struct Core;

impl Core {
    /// Create a new `Core` instance.
    pub fn new() -> Self {
        Core
    }

    /// Submit a natural-language command for processing.
    pub fn submit_command(&self, _text: &str) -> CoreResult<CoreResponse> {
        Err(CoreError::NotImplemented)
    }

    /// Confirm or deny a pending action.
    pub fn confirm_action(
        &self,
        _confirmation_id: &str,
        _decision: ConfirmationDecision,
    ) -> CoreResult<CoreResponse> {
        Err(CoreError::NotImplemented)
    }

    /// Retrieve a snapshot of the current workspace state.
    pub fn get_workspace_snapshot(&self) -> CoreResult<Workspace> {
        Err(CoreError::NotImplemented)
    }

    /// Retrieve recent actions up to `limit`.
    pub fn get_recent_actions(&self, _limit: usize) -> CoreResult<Vec<ActionSummary>> {
        Err(CoreError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_new_returns_instance() {
        let _core = Core::new();
    }

    #[test]
    fn submit_command_returns_not_implemented() {
        let core = Core::new();
        let err = core.submit_command("hello").unwrap_err();
        assert!(matches!(err, CoreError::NotImplemented));
    }

    #[test]
    fn confirm_action_returns_not_implemented() {
        let core = Core::new();
        let err = core
            .confirm_action("id", ConfirmationDecision::Approve)
            .unwrap_err();
        assert!(matches!(err, CoreError::NotImplemented));
    }

    #[test]
    fn get_workspace_snapshot_returns_not_implemented() {
        let core = Core::new();
        let err = core.get_workspace_snapshot().unwrap_err();
        assert!(matches!(err, CoreError::NotImplemented));
    }

    #[test]
    fn get_recent_actions_returns_not_implemented() {
        let core = Core::new();
        let err = core.get_recent_actions(10).unwrap_err();
        assert!(matches!(err, CoreError::NotImplemented));
    }
}
