#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationKind {
    System,
    BuiltIn,
    Custom,
}

pub mod note;
pub mod registry;
pub mod installed;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationAction {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

#[derive(Clone)]
pub struct ApplicationContext {
    pub workspace: std::sync::Arc<crate::workspace::WorkspaceInstance>,
}

impl std::fmt::Debug for ApplicationContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApplicationContext")
            .field("workspace", &self.workspace)
            .finish()
    }
}

#[async_trait::async_trait]
pub trait Application: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn kind(&self) -> ApplicationKind;
    fn tags(&self) -> Vec<String>;
    fn actions(&self) -> Vec<ApplicationAction>;
    async fn execute(
        &self,
        action_id: &str,
        input: serde_json::Value,
        context: &ApplicationContext,
    ) -> crate::error::CoreResult<serde_json::Value>;
}
