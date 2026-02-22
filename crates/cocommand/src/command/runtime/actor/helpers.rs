use serde_json::{Map, Value};

use crate::llm::{LlmTool, LlmToolSet};
use crate::message::ToolState;

pub(super) fn strip_tool_execute(tools: &LlmToolSet) -> LlmToolSet {
    tools
        .iter()
        .map(|(name, tool)| {
            (
                name.clone(),
                LlmTool {
                    description: tool.description.clone(),
                    input_schema: tool.input_schema.clone(),
                    execute: None,
                },
            )
        })
        .collect()
}

pub(super) fn is_async_tool_name(name: &str) -> bool {
    name == "subagent_run" || name == "agent_execute-agent"
}

pub(super) fn running_input_and_start(
    state: &ToolState,
    fallback_start: u64,
) -> (Map<String, Value>, u64) {
    match state {
        ToolState::Pending(state) => (state.input.clone(), fallback_start),
        ToolState::Running(state) => (state.input.clone(), state.time.start),
        ToolState::Completed(state) => (state.input.clone(), state.time.start),
        ToolState::Error(state) => (state.input.clone(), state.time.start),
    }
}

pub(super) fn input_from_tool_state(state: &ToolState) -> Map<String, Value> {
    match state {
        ToolState::Pending(state) => state.input.clone(),
        ToolState::Running(state) => state.input.clone(),
        ToolState::Completed(state) => state.input.clone(),
        ToolState::Error(state) => state.input.clone(),
    }
}
