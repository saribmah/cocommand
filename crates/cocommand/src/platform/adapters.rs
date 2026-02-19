use std::path::Path;
use std::sync::Arc;

use crate::error::{CoreError, CoreResult};

use super::types::{
    ClipboardItem, InstalledApp, PermissionSnapshot, RunningApp, ScreenshotOptions,
    ScreenshotResult, WindowSnapshot,
};

pub trait Platform: Send + Sync {
    fn id(&self) -> &str {
        "unsupported"
    }

    fn list_open_apps(&self, _visible_only: bool) -> CoreResult<Vec<RunningApp>> {
        Err(CoreError::NotImplemented)
    }
    fn list_windows_snapshot(&self, _visible_only: bool) -> CoreResult<WindowSnapshot> {
        Err(CoreError::NotImplemented)
    }
    fn run_applescript(&self, _script: &str) -> CoreResult<String> {
        Err(CoreError::NotImplemented)
    }
    fn list_installed_apps(&self) -> CoreResult<Vec<InstalledApp>> {
        Err(CoreError::NotImplemented)
    }
    fn app_action(
        &self,
        _bundle_id: Option<&str>,
        _pid: Option<i32>,
        _action: &str,
    ) -> CoreResult<()> {
        Err(CoreError::NotImplemented)
    }
    fn window_action(
        &self,
        _window_id: u32,
        _action: &str,
        _snapshot_id: Option<u64>,
    ) -> CoreResult<()> {
        Err(CoreError::NotImplemented)
    }

    fn permissions_snapshot(&self) -> PermissionSnapshot {
        PermissionSnapshot {
            platform: "unsupported".to_string(),
            permissions: Vec::new(),
        }
    }
    fn open_permission_settings(&self, _permission: &str) -> CoreResult<()> {
        Err(CoreError::NotImplemented)
    }

    fn capture_screenshot(
        &self,
        _options: ScreenshotOptions,
        _output_path: Option<&Path>,
    ) -> CoreResult<ScreenshotResult> {
        Err(CoreError::NotImplemented)
    }
    fn supports_screenshot_tools(&self) -> bool {
        false
    }

    fn clipboard_read(&self) -> CoreResult<Option<ClipboardItem>> {
        Err(CoreError::NotImplemented)
    }
    fn clipboard_write(&self, _item: ClipboardItem) -> CoreResult<()> {
        Err(CoreError::NotImplemented)
    }
    fn clipboard_change_count(&self) -> CoreResult<i64> {
        Err(CoreError::NotImplemented)
    }

    fn open_path(&self, _path: &Path) -> CoreResult<()> {
        Err(CoreError::NotImplemented)
    }
    fn reveal_path(&self, _path: &Path) -> CoreResult<()> {
        Err(CoreError::NotImplemented)
    }
    fn icon_of_path(&self, _path: &str) -> Option<String> {
        None
    }
}

pub type SharedPlatform = Arc<dyn Platform>;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(any(not(target_os = "macos"), test))]
pub mod portable;
