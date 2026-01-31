use std::path::Path;
use std::sync::Arc;

use crate::application::Application;
use crate::error::{CoreError, CoreResult};
use crate::extension::application::{tools_from_manifest, ExtensionApplication};
use crate::extension::host::{extension_host_entrypoint, ExtensionHost};
use crate::extension::manifest::ExtensionManifest;

pub async fn load_extension_applications(
    workspace_dir: &Path,
) -> CoreResult<Vec<Arc<dyn Application>>> {
    let extensions_dir = workspace_dir.join("extensions");
    if !extensions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut applications: Vec<Arc<dyn Application>> = Vec::new();
    let host_path = extension_host_entrypoint()?;

    for entry in std::fs::read_dir(&extensions_dir).map_err(|error| {
        CoreError::Internal(format!(
            "failed to read extensions directory {}: {error}",
            extensions_dir.display()
        ))
    })? {
        let entry = entry.map_err(|error| {
            CoreError::Internal(format!("failed to read extension entry: {error}"))
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("manifest.json");
        if !manifest_path.exists() {
            continue;
        }
        let manifest = read_manifest(&manifest_path)?;
        let host = match ExtensionHost::start(&host_path).await {
            Ok(host) => host,
            Err(error) => {
                log::warn!("extension host start failed for {}: {}", manifest.id, error);
                continue;
            }
        };
        let tools = tools_from_manifest(manifest.tools.clone(), None);
        let app = ExtensionApplication::new(manifest, tools, Arc::new(host), path);
        applications.push(Arc::new(app));
    }

    Ok(applications)
}

fn read_manifest(path: &Path) -> CoreResult<ExtensionManifest> {
    let content = std::fs::read_to_string(path).map_err(|error| {
        CoreError::Internal(format!("failed to read extension manifest: {error}"))
    })?;
    serde_json::from_str(&content).map_err(|error| {
        CoreError::Internal(format!("failed to parse extension manifest: {error}"))
    })
}
