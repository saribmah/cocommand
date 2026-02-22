use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ToolExecuteOutput {
    pub title: String,
    pub metadata: Value,
    pub output: Value,
}

impl ToolExecuteOutput {
    pub fn new(title: impl Into<String>, metadata: Value, output: Value) -> Self {
        Self {
            title: title.into(),
            metadata,
            output,
        }
    }

    pub fn with_output(title: impl Into<String>, output: Value) -> Self {
        Self {
            title: title.into(),
            metadata: Value::Object(serde_json::Map::new()),
            output,
        }
    }
}

pub type LlmToolExecute = Arc<
    dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<ToolExecuteOutput, Value>> + Send>>
        + Send
        + Sync,
>;

#[derive(Clone)]
pub struct LlmTool {
    pub description: Option<String>,
    pub input_schema: Value,
    pub execute: Option<LlmToolExecute>,
}

pub type LlmToolSet = HashMap<String, LlmTool>;
