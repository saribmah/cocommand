use serde::{Deserialize, Serialize};

/// Request to submit a natural-language command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitCommandRequest {
    pub text: String,
}

/// Request to confirm or deny a pending action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmActionRequest {
    pub confirmation_id: String,
    pub decision: bool,
}

/// An action that can be attached to an artifact for the UI to present.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactAction {
    pub id: String,
    pub label: String,
}

/// Response returned by core orchestration methods across the Tauri boundary.
///
/// This is the single stable response shape used for all command outcomes.
/// The desktop layer renders UI based solely on which variant it receives.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CoreResponse {
    /// A shell-renderable result with optional actions.
    Artifact {
        content: String,
        actions: Vec<ArtifactAction>,
    },
    /// A read-only preview payload (e.g., last note).
    Preview {
        title: String,
        content: String,
    },
    /// A confirmation prompt for risk actions.
    Confirmation {
        confirmation_id: String,
        prompt: String,
        description: String,
    },
    /// A user-displayable error payload.
    Error {
        message: String,
    },
}

/// A routing candidate (internal use by core orchestration).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutedCandidate {
    pub app_id: String,
    pub score: f64,
    pub explanation: String,
}

/// Summary of a past action for the Recent Actions UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSummary {
    pub id: String,
    pub description: String,
}
