//! Extension manifest types for v0.
//!
//! An extension declares its metadata, routing hints, permissions, and tools
//! in a JSON manifest file loaded at install time.

use serde::{Deserialize, Serialize};

use crate::tools::schema::RiskLevel;

/// Extension manifest (v0).
///
/// Parsed from `manifest.json` in the extension directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    /// Unique extension identifier (e.g., "my-app").
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Short description of the extension.
    pub description: String,
    /// Path to the entrypoint script relative to the extension root.
    pub entrypoint: String,
    /// Routing hints for command matching.
    #[serde(default)]
    pub routing: ExtensionRouting,
    /// Tools provided by this extension.
    #[serde(default)]
    pub tools: Vec<ExtensionToolDef>,
}

/// Routing hints declared in the extension manifest.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtensionRouting {
    /// Keywords that trigger this extension (e.g., ["ticket", "issue"]).
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Example commands (e.g., ["create a ticket"]).
    #[serde(default)]
    pub examples: Vec<String>,
    /// Verbs (e.g., ["create", "update"]).
    #[serde(default)]
    pub verbs: Vec<String>,
    /// Objects/nouns (e.g., ["ticket", "issue"]).
    #[serde(default)]
    pub objects: Vec<String>,
}

/// Tool definition declared in the extension manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionToolDef {
    /// Tool identifier (e.g., "my_app.create_ticket").
    pub id: String,
    /// Risk classification for this tool.
    pub risk_level: RiskLevel,
    /// JSON Schema for input arguments.
    #[serde(default = "default_schema")]
    pub input_schema: serde_json::Value,
    /// JSON Schema for output (documentation only in v0).
    #[serde(default = "default_schema")]
    pub output_schema: serde_json::Value,
}

fn default_schema() -> serde_json::Value {
    serde_json::json!({})
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deserialize_full_manifest() {
        let manifest_json = json!({
            "id": "my-app",
            "name": "My App",
            "description": "A sample extension",
            "entrypoint": "main.ts",
            "routing": {
                "keywords": ["ticket", "issue"],
                "examples": ["create a ticket"],
                "verbs": ["create", "update"],
                "objects": ["ticket"]
            },
            "tools": [
                {
                    "id": "my_app.create_ticket",
                    "risk_level": "Safe",
                    "input_schema": {
                        "type": "object",
                        "required": ["title"],
                        "properties": {
                            "title": {"type": "string"}
                        }
                    },
                    "output_schema": {
                        "type": "object",
                        "properties": {
                            "ticket_id": {"type": "string"}
                        }
                    }
                }
            ]
        });

        let manifest: ExtensionManifest =
            serde_json::from_value(manifest_json).expect("should deserialize");
        assert_eq!(manifest.id, "my-app");
        assert_eq!(manifest.name, "My App");
        assert_eq!(manifest.entrypoint, "main.ts");
        assert_eq!(manifest.routing.keywords, vec!["ticket", "issue"]);
        assert_eq!(manifest.tools.len(), 1);
        assert_eq!(manifest.tools[0].id, "my_app.create_ticket");
        assert_eq!(manifest.tools[0].risk_level, RiskLevel::Safe);
    }

    #[test]
    fn deserialize_minimal_manifest() {
        let manifest_json = json!({
            "id": "bare",
            "name": "Bare Extension",
            "description": "No tools",
            "entrypoint": "index.ts"
        });

        let manifest: ExtensionManifest =
            serde_json::from_value(manifest_json).expect("should deserialize");
        assert_eq!(manifest.id, "bare");
        assert!(manifest.tools.is_empty());
        assert!(manifest.routing.keywords.is_empty());
    }

    #[test]
    fn manifest_roundtrip() {
        let manifest = ExtensionManifest {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "desc".to_string(),
            entrypoint: "main.ts".to_string(),
            routing: ExtensionRouting {
                keywords: vec!["kw".to_string()],
                examples: vec![],
                verbs: vec![],
                objects: vec![],
            },
            tools: vec![ExtensionToolDef {
                id: "test.tool".to_string(),
                risk_level: RiskLevel::Confirm,
                input_schema: json!({}),
                output_schema: json!({}),
            }],
        };

        let serialized = serde_json::to_value(&manifest).unwrap();
        let deserialized: ExtensionManifest =
            serde_json::from_value(serialized).expect("roundtrip");
        assert_eq!(deserialized.id, "test");
        assert_eq!(deserialized.tools[0].risk_level, RiskLevel::Confirm);
    }
}
