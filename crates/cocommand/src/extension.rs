pub mod builtin;
pub mod custom;
pub mod host;
pub mod loader;
pub mod manifest;
pub mod registry;

use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use cocommand_llm::ToolExecuteOutput;
use serde::Serialize;
use serde_json::{json, Value};

use crate::error::CoreResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionStatus {
    Ready,
    Building,
    Error,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionKind {
    System,
    BuiltIn,
    Custom,
}

pub type ExtensionToolExecute = Arc<
    dyn Fn(
            serde_json::Value,
            ExtensionContext,
        ) -> Pin<Box<dyn Future<Output = CoreResult<ToolExecuteOutput>> + Send>>
        + Send
        + Sync,
>;

pub fn boxed_tool_future<F>(
    future: F,
) -> Pin<Box<dyn Future<Output = CoreResult<ToolExecuteOutput>> + Send>>
where
    F: Future<Output = CoreResult<ToolExecuteOutput>> + Send + 'static,
{
    Box::pin(future)
}

pub fn boxed_tool_value_future<F>(
    title: &'static str,
    future: F,
) -> Pin<Box<dyn Future<Output = CoreResult<ToolExecuteOutput>> + Send>>
where
    F: Future<Output = CoreResult<Value>> + Send + 'static,
{
    Box::pin(async move {
        future
            .await
            .map(|output| ToolExecuteOutput::with_output(title, output))
    })
}

#[derive(Clone)]
pub struct ExtensionTool {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
    pub execute: ExtensionToolExecute,
}

pub fn wrap_tool_output_schema(output_schema: Option<Value>) -> Value {
    let metadata_schema = json!({
        "type": "object",
        "default": {},
        "additionalProperties": true
    });
    let output_schema = output_schema.unwrap_or_else(|| json!({}));
    json!({
        "type": "object",
        "properties": {
            "title": { "type": "string" },
            "metadata": metadata_schema,
            "output": output_schema
        },
        "required": ["title", "metadata", "output"],
        "additionalProperties": false
    })
}

#[derive(Clone)]
pub struct ExtensionContext {
    pub workspace: std::sync::Arc<crate::workspace::WorkspaceInstance>,
    pub session_id: String,
}

impl std::fmt::Debug for ExtensionContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExtensionContext")
            .field("workspace", &self.workspace)
            .field("session_id", &self.session_id)
            .finish()
    }
}

/// Context for extension initialization at startup.
#[derive(Clone)]
pub struct ExtensionInitContext {
    pub workspace: std::sync::Arc<crate::workspace::WorkspaceInstance>,
}

#[async_trait::async_trait]
pub trait Extension: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn kind(&self) -> ExtensionKind;
    fn tags(&self) -> Vec<String>;
    fn tools(&self) -> Vec<ExtensionTool>;

    /// Returns self as Any for downcasting to concrete types.
    fn as_any(&self) -> &dyn Any;

    /// Returns the current status of the extension.
    fn status(&self) -> ExtensionStatus {
        ExtensionStatus::Ready
    }

    /// Called once at startup when the extension is registered.
    /// Use this for background tasks like beginning filesystem indexing.
    async fn initialize(&self, _context: ExtensionInitContext) -> crate::error::CoreResult<()> {
        Ok(())
    }

    /// Returns the view configuration if the extension provides a frontend view.
    fn view_config(&self) -> Option<&manifest::ViewConfig> {
        None
    }

    /// Called when the extension is activated in a session context.
    async fn activate(&self, _context: &ExtensionContext) -> crate::error::CoreResult<()> {
        Ok(())
    }
}
