/// A routing candidate exposed in the core response.
#[derive(Debug, Clone)]
pub struct RoutedCandidate {
    pub app_id: String,
    pub score: f64,
    pub explanation: String,
}

/// Response returned by core orchestration methods.
#[derive(Debug, Clone)]
pub enum CoreResponse {
    /// Routing completed, candidates available.
    Routed {
        candidates: Vec<RoutedCandidate>,
        follow_up_active: bool,
    },
    /// Follow-up expired or context missing; user must clarify.
    ClarificationNeeded {
        message: String,
    },
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
