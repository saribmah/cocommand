use std::sync::Arc;

use crate::tool::{LlmTool, LlmToolSet};

/// Convert our `LlmToolSet` into llm-kit's `ToolSet`.
pub fn to_kit_tool_set(tools: LlmToolSet) -> llm_kit_core::tool::ToolSet {
    let mut kit_tools = llm_kit_core::tool::ToolSet::new();
    for (name, tool) in tools {
        kit_tools.insert(name, to_kit_tool(tool));
    }
    kit_tools
}

fn to_kit_tool(tool: LlmTool) -> llm_kit_provider_utils::tool::Tool {
    let mut kit_tool = llm_kit_provider_utils::tool::Tool::function(tool.input_schema);
    if let Some(execute_fn) = tool.execute.clone() {
        let execute = Arc::new(move |input: serde_json::Value, _opts| {
            let execute_fn = execute_fn.clone();
            llm_kit_provider_utils::tool::ToolExecutionOutput::Single(Box::pin(async move {
                execute_fn(input).await
            }))
        });
        kit_tool = kit_tool.with_execute(execute);
    }
    if let Some(description) = tool.description {
        kit_tool = kit_tool.with_description(description);
    }
    kit_tool
}
