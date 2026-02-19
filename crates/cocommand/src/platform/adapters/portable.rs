use std::path::Path;
use std::process::{Command, ExitStatus};

use super::Platform;
use crate::error::{CoreError, CoreResult};
use crate::platform::types::{
    ClipboardItem, InstalledApp, PermissionSnapshot, RunningApp, ScreenshotOptions,
    ScreenshotResult, WindowSnapshot,
};

#[derive(Debug, Default)]
pub struct PortablePlatform;

impl PortablePlatform {
    pub fn new() -> Self {
        Self
    }
}

impl Platform for PortablePlatform {
    fn id(&self) -> &str {
        "unsupported"
    }

    fn list_open_apps(&self, _visible_only: bool) -> CoreResult<Vec<RunningApp>> {
        Err(system_not_supported("list_open_apps"))
    }

    fn list_windows_snapshot(&self, _visible_only: bool) -> CoreResult<WindowSnapshot> {
        Err(system_not_supported("list_windows"))
    }

    fn run_applescript(&self, _script: &str) -> CoreResult<String> {
        Err(system_not_supported("run_applescript"))
    }

    fn list_installed_apps(&self) -> CoreResult<Vec<InstalledApp>> {
        Err(system_not_supported("list_installed_apps"))
    }

    fn app_action(
        &self,
        _bundle_id: Option<&str>,
        _pid: Option<i32>,
        _action: &str,
    ) -> CoreResult<()> {
        Err(system_not_supported("app_action"))
    }

    fn window_action(
        &self,
        _window_id: u32,
        _action: &str,
        _snapshot_id: Option<u64>,
    ) -> CoreResult<()> {
        Err(system_not_supported("window_action"))
    }

    fn permissions_snapshot(&self) -> PermissionSnapshot {
        PermissionSnapshot {
            platform: "unsupported".to_string(),
            permissions: Vec::new(),
        }
    }

    fn open_permission_settings(&self, _permission: &str) -> CoreResult<()> {
        Err(CoreError::Internal("unsupported platform".to_string()))
    }

    fn capture_screenshot(
        &self,
        _options: ScreenshotOptions,
        _output_path: Option<&Path>,
    ) -> CoreResult<ScreenshotResult> {
        Err(CoreError::Internal(
            "screenshot tool not supported on this platform".to_string(),
        ))
    }

    fn supports_screenshot_tools(&self) -> bool {
        false
    }

    fn clipboard_read(&self) -> CoreResult<Option<ClipboardItem>> {
        Err(CoreError::Internal(
            "clipboard not supported on this platform".to_string(),
        ))
    }

    fn clipboard_write(&self, _item: ClipboardItem) -> CoreResult<()> {
        Err(CoreError::Internal(
            "clipboard not supported on this platform".to_string(),
        ))
    }

    fn clipboard_change_count(&self) -> CoreResult<i64> {
        Err(CoreError::Internal(
            "clipboard not supported on this platform".to_string(),
        ))
    }

    fn open_path(&self, path: &Path) -> CoreResult<()> {
        open_path_native(path)
    }

    fn reveal_path(&self, path: &Path) -> CoreResult<()> {
        reveal_path_native(path)
    }

    fn icon_of_path(&self, _path: &str) -> Option<String> {
        None
    }
}

fn system_not_supported(tool_id: &str) -> CoreError {
    CoreError::Internal(format!("system tool not supported: {tool_id}"))
}

fn ensure_command_success(status: ExitStatus, command_label: &str) -> CoreResult<()> {
    if status.success() {
        Ok(())
    } else {
        Err(CoreError::Internal(format!(
            "{command_label} failed with status {status}"
        )))
    }
}

#[cfg(target_os = "macos")]
fn open_path_native(path: &Path) -> CoreResult<()> {
    let status = Command::new("open")
        .arg(path)
        .status()
        .map_err(|error| CoreError::Internal(format!("failed to run open: {error}")))?;
    ensure_command_success(status, "open")
}

#[cfg(target_os = "linux")]
fn open_path_native(path: &Path) -> CoreResult<()> {
    let status = Command::new("xdg-open")
        .arg(path)
        .status()
        .map_err(|error| CoreError::Internal(format!("failed to run xdg-open: {error}")))?;
    ensure_command_success(status, "xdg-open")
}

#[cfg(target_os = "windows")]
fn open_path_native(path: &Path) -> CoreResult<()> {
    let status = Command::new("cmd")
        .arg("/C")
        .arg("start")
        .arg("")
        .arg(path)
        .status()
        .map_err(|error| CoreError::Internal(format!("failed to run start: {error}")))?;
    ensure_command_success(status, "start")
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn open_path_native(_path: &Path) -> CoreResult<()> {
    Err(CoreError::Internal(
        "open_path is not supported on this platform".to_string(),
    ))
}

#[cfg(target_os = "macos")]
fn reveal_path_native(path: &Path) -> CoreResult<()> {
    let status = Command::new("open")
        .arg("-R")
        .arg(path)
        .status()
        .map_err(|error| CoreError::Internal(format!("failed to reveal path: {error}")))?;
    ensure_command_success(status, "open -R")
}

#[cfg(not(target_os = "macos"))]
fn reveal_path_native(path: &Path) -> CoreResult<()> {
    let parent = path.parent().unwrap_or(path);
    open_path_native(parent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_system_tools_return_expected_error_shape() {
        let adapter = PortablePlatform::new();
        let error = adapter
            .list_open_apps(false)
            .expect_err("list_open_apps should be unsupported");
        match error {
            CoreError::Internal(message) => {
                assert_eq!(message, "system tool not supported: list_open_apps")
            }
            other => panic!("unexpected error variant: {other}"),
        }
    }

    #[test]
    fn unsupported_screenshot_returns_expected_message() {
        let adapter = PortablePlatform::new();
        let result = adapter.capture_screenshot(
            ScreenshotOptions {
                mode: crate::platform::types::ScreenshotMode::Screen,
                display: None,
                window_id: None,
                rect: None,
                format: Some("png".to_string()),
                delay_seconds: None,
                to_clipboard: false,
                include_cursor: false,
            },
            None,
        );
        assert!(matches!(
            result,
            Err(CoreError::Internal(message)) if message == "screenshot tool not supported on this platform"
        ));
    }
}
