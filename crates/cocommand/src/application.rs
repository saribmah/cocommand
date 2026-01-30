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
}

pub trait Application: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn kind(&self) -> ApplicationKind;
    fn tags(&self) -> Vec<String>;
    fn actions(&self) -> Vec<ApplicationAction>;
}
