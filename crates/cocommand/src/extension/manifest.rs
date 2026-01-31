use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub entrypoint: String,
    pub routing: Option<ExtensionRouting>,
    pub tools: Option<Vec<ExtensionTool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionRouting {
    pub keywords: Option<Vec<String>>,
    pub examples: Option<Vec<String>>,
    pub verbs: Option<Vec<String>>,
    pub objects: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionTool {
    pub id: String,
    pub risk_level: String,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
}
