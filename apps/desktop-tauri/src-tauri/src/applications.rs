pub mod spotify;

use serde::Serialize;

pub trait Application {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn tools(&self) -> Vec<ToolDefinition>;
}

pub trait Tool {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
}

#[derive(Clone, Serialize)]
pub struct ToolDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Serialize)]
pub struct ApplicationDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tools: Vec<ToolDefinition>,
}

pub fn all_apps() -> Vec<ApplicationDefinition> {
    let apps: Vec<Box<dyn Application>> = vec![Box::new(spotify::SpotifyApp::default())];
    apps.into_iter()
        .map(|app| ApplicationDefinition {
            id: app.id().to_string(),
            name: app.name().to_string(),
            description: app.description().to_string(),
            tools: app.tools(),
        })
        .collect()
}

pub fn all_tools() -> Vec<ToolDefinition> {
    all_apps()
        .into_iter()
        .flat_map(|app| app.tools)
        .collect()
}

pub fn tool_definition<T: Tool>(tool: &T) -> ToolDefinition {
    ToolDefinition {
        id: tool.id().to_string(),
        name: tool.name().to_string(),
        description: tool.description().to_string(),
    }
}
