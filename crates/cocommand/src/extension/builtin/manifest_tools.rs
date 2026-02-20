use std::collections::HashMap;

use crate::extension::manifest::ExtensionManifest;
use crate::extension::{ExtensionTool, ExtensionToolExecute};

/// All builtin manifest JSON strings, embedded at compile time.
pub const BUILTIN_MANIFESTS: &[&str] = &[
    include_str!("clipboard_manifest.json"),
    include_str!("browser_manifest.json"),
    include_str!("workspace_manifest.json"),
    include_str!("screenshot_manifest.json"),
    include_str!("system_manifest.json"),
    include_str!("note/manifest.json"),
    include_str!("filesystem/manifest.json"),
    include_str!("agent/manifest.json"),
];

/// Parse a builtin manifest from embedded JSON.
pub fn parse_builtin_manifest(json: &str) -> ExtensionManifest {
    serde_json::from_str(json).expect("builtin manifest JSON must be valid")
}

/// Returns all builtin manifests parsed.
pub fn all_builtin_manifests() -> Vec<ExtensionManifest> {
    BUILTIN_MANIFESTS
        .iter()
        .map(|json| parse_builtin_manifest(json))
        .collect()
}

/// Merge manifest tool metadata with Rust execute closures.
///
/// For each tool in the manifest, looks up the corresponding execute closure
/// in the map. Tools without a matching closure are skipped (allows
/// platform-specific tools to be omitted on unsupported platforms).
pub fn merge_manifest_tools(
    manifest: &ExtensionManifest,
    execute_map: HashMap<&str, ExtensionToolExecute>,
) -> Vec<ExtensionTool> {
    let manifest_tools = match &manifest.tools {
        Some(tools) => tools,
        None => return Vec::new(),
    };

    let mut result = Vec::with_capacity(manifest_tools.len());

    for mt in manifest_tools {
        let execute = match execute_map.get(mt.id.as_str()) {
            Some(exec) => exec.clone(),
            None => continue,
        };

        result.push(ExtensionTool {
            id: mt.id.clone(),
            name: mt.name.clone().unwrap_or_else(|| mt.id.clone()),
            description: mt.description.clone(),
            input_schema: mt
                .input_schema
                .clone()
                .unwrap_or_else(|| serde_json::json!({ "type": "object" })),
            output_schema: mt.output_schema.clone(),
            execute,
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_manifest_parses() {
        let manifest = parse_builtin_manifest(include_str!("clipboard_manifest.json"));
        assert_eq!(manifest.id, "clipboard");
        assert_eq!(manifest.name, "Clipboard");
        assert!(manifest.entrypoint.is_none());
        let tools = manifest.tools.as_ref().unwrap();
        assert_eq!(tools.len(), 5);
        assert_eq!(tools[0].id, "get_clipboard");
        assert!(tools[0].name.is_some());
        assert!(tools[0].description.is_some());
    }

    #[test]
    fn browser_manifest_parses() {
        let manifest = parse_builtin_manifest(include_str!("browser_manifest.json"));
        assert_eq!(manifest.id, "browser");
        let tools = manifest.tools.as_ref().unwrap();
        assert_eq!(tools.len(), 3);
    }

    #[test]
    fn workspace_manifest_parses() {
        let manifest = parse_builtin_manifest(include_str!("workspace_manifest.json"));
        assert_eq!(manifest.id, "workspace");
        let tools = manifest.tools.as_ref().unwrap();
        assert_eq!(tools.len(), 4);
    }

    #[test]
    fn screenshot_manifest_parses() {
        let manifest = parse_builtin_manifest(include_str!("screenshot_manifest.json"));
        assert_eq!(manifest.id, "screenshot");
        let tools = manifest.tools.as_ref().unwrap();
        assert_eq!(tools.len(), 5);
    }

    #[test]
    fn system_manifest_parses() {
        let manifest = parse_builtin_manifest(include_str!("system_manifest.json"));
        assert_eq!(manifest.id, "system");
        let tools = manifest.tools.as_ref().unwrap();
        assert_eq!(tools.len(), 6);
    }

    #[test]
    fn note_manifest_parses() {
        let manifest = parse_builtin_manifest(include_str!("note/manifest.json"));
        assert_eq!(manifest.id, "notes");
        let tools = manifest.tools.as_ref().unwrap();
        assert_eq!(tools.len(), 8);
    }

    #[test]
    fn filesystem_manifest_parses() {
        let manifest = parse_builtin_manifest(include_str!("filesystem/manifest.json"));
        assert_eq!(manifest.id, "filesystem");
        let tools = manifest.tools.as_ref().unwrap();
        assert_eq!(tools.len(), 9);
    }

    #[test]
    fn agent_manifest_parses() {
        let manifest = parse_builtin_manifest(include_str!("agent/manifest.json"));
        assert_eq!(manifest.id, "agent");
        let tools = manifest.tools.as_ref().unwrap();
        assert_eq!(tools.len(), 6);
    }
}
