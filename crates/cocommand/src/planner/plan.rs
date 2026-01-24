/// A single tool call within a plan.
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedToolCall {
    pub tool_id: String,
    pub args: serde_json::Value,
}

/// An ordered sequence of tool calls produced by the planner.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub steps: Vec<PlannedToolCall>,
}

impl Plan {
    pub fn new(steps: Vec<PlannedToolCall>) -> Self {
        Self { steps }
    }

    pub fn empty() -> Self {
        Self { steps: Vec::new() }
    }
}
