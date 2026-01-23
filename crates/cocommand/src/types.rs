/// Response returned by core orchestration methods.
#[derive(Debug, Clone)]
pub struct CoreResponse {
    pub message: String,
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
