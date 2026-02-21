use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;

pub type LlmToolExecute =
    Arc<dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<Value, Value>> + Send>> + Send + Sync>;

#[derive(Clone)]
pub struct LlmTool {
    pub description: Option<String>,
    pub input_schema: Value,
    pub execute: Option<LlmToolExecute>,
}

pub type LlmToolSet = HashMap<String, LlmTool>;
