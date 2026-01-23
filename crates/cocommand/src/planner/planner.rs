use crate::command::ParsedCommand;
use crate::routing::RouteCandidate;
use super::plan::{Plan, PlannedToolCall};

/// Trait for planning tool call sequences from a command and routing candidates.
pub trait Planner {
    fn plan(&self, command: &ParsedCommand, candidates: &[RouteCandidate]) -> Plan;
}

/// Deterministic stub planner for v0.
///
/// If candidates is non-empty, returns a single-step plan using the first
/// candidate's `app_id` as a tool_id prefix and the command's `normalized_text`
/// as an arg. Otherwise returns an empty plan.
pub struct StubPlanner;

impl Planner for StubPlanner {
    fn plan(&self, command: &ParsedCommand, candidates: &[RouteCandidate]) -> Plan {
        match candidates.first() {
            Some(candidate) => {
                let step = PlannedToolCall {
                    tool_id: format!("{}.execute", candidate.app_id),
                    args: serde_json::json!({ "input": command.normalized_text }),
                };
                Plan::new(vec![step])
            }
            None => Plan::empty(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn stub_returns_empty_plan_for_no_candidates() {
        let planner = StubPlanner;
        let cmd = make_command("do something");
        let plan = planner.plan(&cmd, &[]);

        assert_eq!(plan, Plan::empty());
        assert!(plan.steps.is_empty());
    }

    #[test]
    fn stub_returns_single_step_for_one_candidate() {
        let planner = StubPlanner;
        let cmd = make_command("copy text");
        let candidates = vec![make_candidate("clipboard", 5.0)];
        let plan = planner.plan(&cmd, &candidates);

        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].tool_id, "clipboard.execute");
        assert_eq!(plan.steps[0].args, serde_json::json!({ "input": "copy text" }));
    }

    #[test]
    fn stub_output_is_deterministic() {
        let planner = StubPlanner;
        let cmd = make_command("open file");
        let candidates = vec![
            make_candidate("files", 6.0),
            make_candidate("editor", 3.0),
        ];

        let plan1 = planner.plan(&cmd, &candidates);
        let plan2 = planner.plan(&cmd, &candidates);

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
