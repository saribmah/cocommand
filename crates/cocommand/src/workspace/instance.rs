use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::application::note::NoteApplication;
use crate::application::registry::ApplicationRegistry;
use crate::application::Application;
use crate::application::installed::InstalledApplication;
use crate::error::{CoreError, CoreResult};
use crate::storage::file::FileStorage;
use crate::storage::SharedStorage;
use crate::workspace::config::{
    load_or_create_workspace_storage, WorkspaceConfig,
};

#[derive(Clone)]
pub struct WorkspaceInstance {
    pub workspace_dir: PathBuf,
    pub config: Arc<RwLock<WorkspaceConfig>>,
    pub application_registry: Arc<RwLock<ApplicationRegistry>>,
    pub storage: SharedStorage,
}

impl fmt::Debug for WorkspaceInstance {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let config = self.config.blocking_read();
        formatter
            .debug_struct("WorkspaceInstance")
            .field("workspace_dir", &self.workspace_dir)
            .field("config", &*config)
            .finish()
    }
}

impl WorkspaceInstance {
    pub async fn new(workspace_dir: &Path) -> CoreResult<Self> {
        if !workspace_dir.exists() {
            std::fs::create_dir_all(workspace_dir).map_err(|error| {
                CoreError::Internal(format!(
                    "failed to create workspace directory {}: {error}",
                    workspace_dir.display()
                ))
            })?;
        }
        let storage_root = workspace_dir.join("storage");
        let storage: SharedStorage = Arc::new(FileStorage::new(storage_root));
        let config = load_or_create_workspace_storage(storage.as_ref()).await?;
        let application_registry = Arc::new(RwLock::new(ApplicationRegistry::new()));
        register_builtin_applications(&application_registry).await;
        Ok(Self {
            workspace_dir: workspace_dir.to_path_buf(),
            config: Arc::new(RwLock::new(config)),
            application_registry,
            storage,
        })
    }
}

async fn register_builtin_applications(registry: &Arc<RwLock<ApplicationRegistry>>) {
    let mut registry = registry.write().await;
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
