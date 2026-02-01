#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationKind {
    System,
    BuiltIn,
    Custom,
}

pub mod note;
pub mod registry;
pub mod system;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationTool {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

#[derive(Clone)]
pub struct ApplicationContext {
    pub workspace: std::sync::Arc<crate::workspace::WorkspaceInstance>,
    pub session_id: String,
}

impl std::fmt::Debug for ApplicationContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApplicationContext")
            .field("workspace", &self.workspace)
            .field("session_id", &self.session_id)
            .finish()
    }
}

#[async_trait::async_trait]
pub trait Application: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn kind(&self) -> ApplicationKind;
    fn tags(&self) -> Vec<String>;
    fn tools(&self) -> Vec<ApplicationTool>;
    async fn initialize(&self, _context: &ApplicationContext) -> crate::error::CoreResult<()> {
        Ok(())
    }
    async fn execute(
        &self,
        tool_id: &str,
        input: serde_json::Value,
        context: &ApplicationContext,
    ) -> crate::error::CoreResult<serde_json::Value>;
}
