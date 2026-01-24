use async_trait::async_trait;

use super::plan::{Plan, PlannedToolCall};
use super::types::{PlanMetadata, PlannerError, PlannerInput, PlannerOutput};

/// Trait for planning tool call sequences from a command and routing candidates.
#[async_trait]
pub trait Planner: Send + Sync {
    async fn plan(&self, input: PlannerInput) -> Result<PlannerOutput, PlannerError>;
}

/// Deterministic stub planner for v0.
///
/// If candidates is non-empty, returns a single-step plan using the first
/// candidate's `app_id` as a tool_id prefix and the command's `normalized_text`
/// as an arg. Otherwise returns an empty plan.
pub struct StubPlanner;

#[async_trait]
impl Planner for StubPlanner {
    async fn plan(&self, input: PlannerInput) -> Result<PlannerOutput, PlannerError> {
        let plan = match input.candidates.first() {
            Some(candidate) => {
                let step = PlannedToolCall {
                    tool_id: format!("{}.execute", candidate.app_id),
                    args: serde_json::json!({ "input": input.command.normalized_text }),
                };
                Plan::new(vec![step])
            }
            None => Plan::empty(),
        };

        Ok(PlannerOutput::new(plan, PlanMetadata::stub()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::ParsedCommand;
    use crate::routing::RouteCandidate;
    use crate::workspace::Workspace;

    fn make_command(text: &str) -> ParsedCommand {
        ParsedCommand {
            raw_text: text.to_string(),
            normalized_text: text.to_string(),
            tags: vec![],
        }
    }

    fn make_candidate(app_id: &str, score: f64) -> RouteCandidate {
        RouteCandidate {
            app_id: app_id.to_string(),
            score,
            explanation: "test".to_string(),
        }
    }

    fn make_input(candidates: Vec<RouteCandidate>) -> PlannerInput {
        PlannerInput {
            command: make_command("do something"),
            candidates,
            workspace: Workspace::new("test-session".to_string()),
            tools: vec![],
        }
    }

    #[tokio::test]
    async fn stub_returns_empty_plan_for_no_candidates() {
        let planner = StubPlanner;
        let output = planner.plan(make_input(vec![])).await.unwrap();

        assert_eq!(output.plan, Plan::empty());
        assert!(output.plan.steps.is_empty());
    }

    #[tokio::test]
    async fn stub_returns_single_step_for_one_candidate() {
        let planner = StubPlanner;
        let candidates = vec![make_candidate("clipboard", 5.0)];
        let mut input = make_input(candidates);
        input.command = make_command("copy text");
        let output = planner.plan(input).await.unwrap();

        assert_eq!(output.plan.steps.len(), 1);
        assert_eq!(output.plan.steps[0].tool_id, "clipboard.execute");
        assert_eq!(
            output.plan.steps[0].args,
            serde_json::json!({ "input": "copy text" })
        );
    }

    #[tokio::test]
    async fn stub_output_is_deterministic() {
        let planner = StubPlanner;
        let candidates = vec![
            make_candidate("files", 6.0),
            make_candidate("editor", 3.0),
        ];

        let mut input = make_input(candidates);
        input.command = make_command("open file");
        let plan1 = planner.plan(input.clone()).await.unwrap();
        let plan2 = planner.plan(input).await.unwrap();

        assert_eq!(plan1, plan2);
    }

    #[test]
    fn plan_preserves_step_ordering() {
        let steps = vec![
            PlannedToolCall {
                tool_id: "first.execute".to_string(),
                args: serde_json::json!({ "order": 1 }),
            },
            PlannedToolCall {
                tool_id: "second.execute".to_string(),
                args: serde_json::json!({ "order": 2 }),
            },
            PlannedToolCall {
                tool_id: "third.execute".to_string(),
                args: serde_json::json!({ "order": 3 }),
            },
        ];

        let plan = Plan::new(steps.clone());

        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.steps[0].tool_id, "first.execute");
        assert_eq!(plan.steps[1].tool_id, "second.execute");
        assert_eq!(plan.steps[2].tool_id, "third.execute");
    }

    #[test]
    fn plan_empty_has_no_steps() {
        let plan = Plan::empty();
        assert!(plan.steps.is_empty());
        assert_eq!(plan.steps.len(), 0);
    }
}
