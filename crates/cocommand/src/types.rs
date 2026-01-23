/// Response returned by core orchestration methods.
#[derive(Debug, Clone)]
pub struct CoreResponse {
    pub message: String,
}

/// Snapshot of the current workspace state (placeholder for Core-1).
#[derive(Debug, Clone)]
pub struct Workspace {
    pub active_apps: Vec<String>,
}

/// Summary of a past action (placeholder for Core-2).
#[derive(Debug, Clone)]
pub struct ActionSummary {
    pub id: String,
    pub description: String,
}

/// User's confirmation decision.
#[derive(Debug, Clone)]
pub enum ConfirmationDecision {
    Approve,
    Deny,
}
