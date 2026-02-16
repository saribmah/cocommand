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

use crate::error::CoreResult;

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
        ) -> Pin<Box<dyn Future<Output = CoreResult<serde_json::Value>> + Send>>
        + Send
        + Sync,
>;

pub fn boxed_tool_future<F>(
    future: F,
) -> Pin<Box<dyn Future<Output = CoreResult<serde_json::Value>> + Send>>
where
    F: Future<Output = CoreResult<serde_json::Value>> + Send + 'static,
{
    Box::pin(future)
}

#[derive(Clone)]
pub struct ExtensionTool {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
    pub execute: ExtensionToolExecute,
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
