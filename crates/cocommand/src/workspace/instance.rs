use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{CoreError, CoreResult};
use crate::extension::builtin::clipboard::ClipboardExtension;
use crate::extension::builtin::note::NoteExtension;
use crate::extension::builtin::screenshot::ScreenshotExtension;
use crate::extension::builtin::system::SystemExtension;
use crate::extension::loader::load_custom_extensions;
use crate::extension::registry::ExtensionRegistry;
use crate::extension::Extension;
use crate::storage::file::FileStorage;
use crate::storage::SharedStorage;
use crate::workspace::config::{load_or_create_workspace_storage, WorkspaceConfig};

#[derive(Clone)]
pub struct WorkspaceInstance {
    pub workspace_dir: PathBuf,
    pub config: Arc<RwLock<WorkspaceConfig>>,
    pub extension_registry: Arc<RwLock<ExtensionRegistry>>,
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
        let extension_registry = Arc::new(RwLock::new(ExtensionRegistry::new()));
        register_builtin_extensions(&extension_registry).await;
        register_custom_extensions(&extension_registry, workspace_dir).await?;
        Ok(Self {
            workspace_dir: workspace_dir.to_path_buf(),
            config: Arc::new(RwLock::new(config)),
            extension_registry,
            storage,
        })
    }
}

async fn register_builtin_extensions(registry: &Arc<RwLock<ExtensionRegistry>>) {
    let mut registry = registry.write().await;
    registry.register(Arc::new(ClipboardExtension::new()) as Arc<dyn Extension>);
    registry.register(Arc::new(NoteExtension::new()) as Arc<dyn Extension>);
    registry.register(Arc::new(SystemExtension::new()) as Arc<dyn Extension>);
    registry.register(Arc::new(ScreenshotExtension::new()) as Arc<dyn Extension>);
}

async fn register_custom_extensions(
    registry: &Arc<RwLock<ExtensionRegistry>>,
    workspace_dir: &Path,
) -> CoreResult<()> {
    let apps = load_custom_extensions(workspace_dir).await?;
    if apps.is_empty() {
        return Ok(());
    }
    let mut registry = registry.write().await;
    for app in apps {
        registry.register(app);
    }
    Ok(())
}
