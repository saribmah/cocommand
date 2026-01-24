use crate::command::ParsedCommand;
use crate::routing::RouteCandidate;
use crate::tools::RiskLevel;
use crate::workspace::Workspace;

/// Minimal tool metadata exposed to planners.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolSpec {
    pub tool_id: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub risk_level: RiskLevel,
    pub is_kernel: bool,
}

/// Structured input to a planner.
#[derive(Debug, Clone)]
pub struct PlannerInput {
    pub command: ParsedCommand,
    pub candidates: Vec<RouteCandidate>,
    pub workspace: Workspace,
    pub tools: Vec<ToolSpec>,
}

/// Metadata about how a plan was produced.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanMetadata {
    pub planner_id: String,
    pub model: Option<String>,
    pub reasoning: Option<String>,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

impl PlanMetadata {
    pub fn stub() -> Self {
        Self {
            planner_id: "stub".to_string(),
            model: None,
            reasoning: None,
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
        }
    }
}

/// Planner output containing a tool-call plan plus metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct PlannerOutput {
    pub plan: super::plan::Plan,
    pub metadata: PlanMetadata,
}

impl PlannerOutput {
    pub fn new(plan: super::plan::Plan, metadata: PlanMetadata) -> Self {
        Self { plan, metadata }
    }
}

/// Planner error types.
#[derive(Debug, Clone, PartialEq)]
pub enum PlannerError {
    ProviderUnavailable(String),
    InvalidResponse(String),
    Internal(String),
}
