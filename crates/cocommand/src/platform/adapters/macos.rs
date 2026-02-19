use std::path::Path;
use std::process::{Command, ExitStatus};

use super::Platform;
use crate::error::{CoreError, CoreResult};
use crate::platform::types::{
    ClipboardItem, InstalledApp, PermissionSnapshot, PermissionStatus, Rect, RunningApp,
    ScreenshotMode, ScreenshotOptions, ScreenshotResult, WindowInfo, WindowKind, WindowSnapshot,
    WindowState,
};

#[derive(Debug, Default)]
pub struct MacosPlatform;

impl MacosPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl Platform for MacosPlatform {
    fn id(&self) -> &str {
        "macos"
    }

    fn list_open_apps(&self, visible_only: bool) -> CoreResult<Vec<RunningApp>> {
        let apps = platform_macos::list_open_apps(visible_only).map_err(CoreError::Internal)?;
        Ok(apps.into_iter().map(map_running_app).collect())
    }

    fn list_windows_snapshot(&self, visible_only: bool) -> CoreResult<WindowSnapshot> {
        let snapshot =
            platform_macos::list_windows_snapshot(visible_only).map_err(CoreError::Internal)?;
        Ok(WindowSnapshot {
            snapshot_id: snapshot.snapshot_id,
            windows: snapshot.windows.into_iter().map(map_window_info).collect(),
        })
    }

    fn run_applescript(&self, script: &str) -> CoreResult<String> {
        platform_macos::run_applescript(script).map_err(CoreError::Internal)
    }

    fn list_installed_apps(&self) -> CoreResult<Vec<InstalledApp>> {
        let apps = platform_macos::list_installed_apps();
        Ok(apps
            .into_iter()
            .map(|app| InstalledApp {
                name: app.name,
                bundle_id: app.bundle_id,
                path: app.path,
                icon: app.icon,
            })
            .collect())
    }

    fn app_action(
        &self,
        bundle_id: Option<&str>,
        pid: Option<i32>,
        action: &str,
    ) -> CoreResult<()> {
        platform_macos::perform_app_action(bundle_id, pid, action).map_err(CoreError::Internal)
    }

    fn window_action(
        &self,
        window_id: u32,
        action: &str,
        snapshot_id: Option<u64>,
    ) -> CoreResult<()> {
        platform_macos::perform_window_action(window_id, action, snapshot_id)
            .map_err(CoreError::Internal)
    }

    fn permissions_snapshot(&self) -> PermissionSnapshot {
        PermissionSnapshot {
            platform: "macos".to_string(),
            permissions: vec![
                PermissionStatus {
                    id: "accessibility".to_string(),
                    label: "Accessibility".to_string(),
                    granted: platform_macos::check_accessibility(),
                    required: true,
                },
                PermissionStatus {
                    id: "screen-recording".to_string(),
                    label: "Screen Recording".to_string(),
                    granted: platform_macos::check_screen_recording(),
                    required: true,
                },
                PermissionStatus {
                    id: "automation".to_string(),
                    label: "Automation".to_string(),
                    granted: platform_macos::check_automation().unwrap_or(false),
                    required: true,
                },
            ],
        }
    }

    fn open_permission_settings(&self, permission: &str) -> CoreResult<()> {
        platform_macos::open_permission_settings(permission).map_err(CoreError::Internal)
    }

    fn capture_screenshot(
        &self,
        options: ScreenshotOptions,
        output_path: Option<&Path>,
    ) -> CoreResult<ScreenshotResult> {
        let options = platform_macos::ScreenshotOptions {
            mode: map_screenshot_mode(options.mode),
            display: options.display,
            window_id: options.window_id,
            rect: options.rect,
            format: options.format,
            delay_seconds: options.delay_seconds,
            to_clipboard: options.to_clipboard,
            include_cursor: options.include_cursor,
        };
        let result = platform_macos::capture_screenshot(options, output_path)
            .map_err(CoreError::Internal)?;
        Ok(ScreenshotResult {
            path: result.path,
            filename: result.filename,
            format: result.format,
            clipboard: result.clipboard,
        })
    }

    fn supports_screenshot_tools(&self) -> bool {
        true
    }

    fn clipboard_read(&self) -> CoreResult<Option<ClipboardItem>> {
        let item = platform_macos::read_clipboard().map_err(CoreError::Internal)?;
        Ok(item.map(map_clipboard_item))
    }

    fn clipboard_write(&self, item: ClipboardItem) -> CoreResult<()> {
        let item = match item {
            ClipboardItem::Text(text) => platform_macos::ClipboardItem::Text(text),
            ClipboardItem::Image(bytes) => platform_macos::ClipboardItem::Image(bytes),
            ClipboardItem::Files(files) => platform_macos::ClipboardItem::Files(files),
        };
        platform_macos::write_clipboard(item).map_err(CoreError::Internal)
    }

    fn clipboard_change_count(&self) -> CoreResult<i64> {
        platform_macos::clipboard_change_count().map_err(CoreError::Internal)
    }

    fn open_path(&self, path: &Path) -> CoreResult<()> {
        let status = Command::new("open")
            .arg(path)
            .status()
            .map_err(|error| CoreError::Internal(format!("failed to run open: {error}")))?;
        ensure_command_success(status, "open")
    }

    fn reveal_path(&self, path: &Path) -> CoreResult<()> {
        let status = Command::new("open")
            .arg("-R")
            .arg(path)
            .status()
            .map_err(|error| CoreError::Internal(format!("failed to reveal path: {error}")))?;
        ensure_command_success(status, "open -R")
    }

    fn icon_of_path(&self, path: &str) -> Option<String> {
        platform_macos::icon_of_path(path)
    }
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

fn map_clipboard_item(item: platform_macos::ClipboardItem) -> ClipboardItem {
    match item {
        platform_macos::ClipboardItem::Text(text) => ClipboardItem::Text(text),
        platform_macos::ClipboardItem::Image(bytes) => ClipboardItem::Image(bytes),
        platform_macos::ClipboardItem::Files(files) => ClipboardItem::Files(files),
    }
}

fn map_screenshot_mode(mode: ScreenshotMode) -> platform_macos::ScreenshotMode {
    match mode {
        ScreenshotMode::Interactive => platform_macos::ScreenshotMode::Interactive,
        ScreenshotMode::Screen => platform_macos::ScreenshotMode::Screen,
        ScreenshotMode::Window => platform_macos::ScreenshotMode::Window,
        ScreenshotMode::Rect => platform_macos::ScreenshotMode::Rect,
    }
}

fn map_running_app(app: platform_macos::RunningApp) -> RunningApp {
    RunningApp {
        name: app.name,
        bundle_id: app.bundle_id,
        pid: app.pid,
        is_active: app.is_active,
        is_hidden: app.is_hidden,
        has_windows: app.has_windows,
        windows: app.windows.into_iter().map(map_window_info).collect(),
    }
}

fn map_rect(rect: platform_macos::Rect) -> Rect {
    Rect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    }
}

fn map_window_state(state: platform_macos::WindowState) -> WindowState {
    match state {
        platform_macos::WindowState::Onscreen => WindowState::Onscreen,
        platform_macos::WindowState::Offscreen => WindowState::Offscreen,
        platform_macos::WindowState::Minimized => WindowState::Minimized,
        platform_macos::WindowState::Unknown => WindowState::Unknown,
    }
}

fn map_window_kind(kind: platform_macos::WindowKind) -> WindowKind {
    match kind {
        platform_macos::WindowKind::Normal => WindowKind::Normal,
        platform_macos::WindowKind::Unknown => WindowKind::Unknown,
    }
}

fn map_window_info(window: platform_macos::WindowInfo) -> WindowInfo {
    WindowInfo {
        window_id: window.window_id,
        title: window.title,
        bounds: map_rect(window.bounds),
        layer: window.layer,
        is_onscreen: window.is_onscreen,
        minimized: window.minimized,
        state: map_window_state(window.state),
        kind: map_window_kind(window.kind),
        owner_pid: window.owner_pid,
        owner_name: window.owner_name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_snapshot_has_expected_shape() {
        let adapter = MacosPlatform::new();
        let snapshot = adapter.permissions_snapshot();
        assert_eq!(snapshot.platform, "macos");
        assert_eq!(snapshot.permissions.len(), 3);
        assert_eq!(snapshot.permissions[0].id, "accessibility");
        assert_eq!(snapshot.permissions[1].id, "screen-recording");
        assert_eq!(snapshot.permissions[2].id, "automation");
    }
}
