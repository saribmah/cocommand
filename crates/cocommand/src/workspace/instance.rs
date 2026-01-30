use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::application::note::NoteApplication;
use crate::application::registry::ApplicationRegistry;
use crate::application::Application;
use crate::application::installed::InstalledApplication;
use crate::error::{CoreError, CoreResult};
use crate::workspace::config::{load_or_create_workspace_config, WorkspaceConfig};

#[derive(Clone)]
pub struct WorkspaceInstance {
    pub workspace_dir: PathBuf,
    pub config: WorkspaceConfig,
    pub application_registry: Arc<RwLock<ApplicationRegistry>>,
}

impl fmt::Debug for WorkspaceInstance {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WorkspaceInstance")
            .field("workspace_dir", &self.workspace_dir)
            .field("config", &self.config)
            .finish()
    }
}

impl WorkspaceInstance {
    pub fn new(workspace_dir: &Path) -> CoreResult<Self> {
        if !workspace_dir.exists() {
            std::fs::create_dir_all(workspace_dir).map_err(|error| {
                CoreError::Internal(format!(
                    "failed to create workspace directory {}: {error}",
                    workspace_dir.display()
                ))
            })?;
        }
        let config = load_or_create_workspace_config(workspace_dir)?;
        let application_registry = Arc::new(RwLock::new(ApplicationRegistry::new()));
        register_builtin_applications(&application_registry);
        Ok(Self {
            workspace_dir: workspace_dir.to_path_buf(),
            config,
            application_registry,
        })
    }
}

fn register_builtin_applications(registry: &Arc<RwLock<ApplicationRegistry>>) {
    let mut registry = registry
        .write()
        .expect("failed to acquire application registry write lock");
    registry.register(Arc::new(NoteApplication::new()) as Arc<dyn Application>);
    register_installed_applications(&mut registry);
}

fn register_installed_applications(registry: &mut ApplicationRegistry) {
    #[cfg(target_os = "macos")]
    {
        use platform_macos::list_installed_apps;
        for app in list_installed_apps() {
            let id = app.bundle_id.clone().unwrap_or_else(|| app.path.clone());
            let installed = InstalledApplication::new(id, app.name, app.bundle_id, app.path);
            registry.register(Arc::new(installed) as Arc<dyn Application>);
        }
    }
}
