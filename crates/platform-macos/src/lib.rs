mod applescript;
mod application;
mod installed;
mod screenshot;
mod util;
mod window;

pub use applescript::run_applescript;
pub use application::{list_open_apps, perform_app_action, RunningApp};
pub use installed::{
    execute_installed_app_tool, installed_app_tools, list_installed_apps, open_installed_app,
    InstalledApp,
};
pub use screenshot::{capture_screenshot, ScreenshotMode, ScreenshotOptions, ScreenshotResult};
pub use window::{perform_window_action, WindowInfo};
