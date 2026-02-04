mod applescript;
mod application;
mod clipboard;
mod installed;
mod permissions;
mod screen;
mod screenshot;
mod util;
mod window;

pub use applescript::run_applescript;
pub use application::{list_open_apps, perform_app_action, RunningApp};
pub use clipboard::{clipboard_change_count, read_clipboard, write_clipboard, ClipboardItem};
pub use installed::{
    execute_installed_app_tool, installed_app_tools, list_installed_apps, open_installed_app,
    InstalledApp,
};
pub use permissions::{
    check_accessibility, check_automation, check_screen_recording, open_permission_settings,
};
pub use screen::{active_screen_visible_frame, ScreenFrame};
pub use screenshot::{capture_screenshot, ScreenshotMode, ScreenshotOptions, ScreenshotResult};
pub use window::{
    get_windows_snapshot, list_windows, list_windows_snapshot, perform_window_action, Rect,
    WindowInfo, WindowKind, WindowState,
};
