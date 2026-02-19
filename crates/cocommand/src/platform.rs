mod adapters;
pub mod factory;
pub mod types;

pub use adapters::{Platform, SharedPlatform};
pub use factory::default_platform;
pub use types::{
    ClipboardItem, InstalledApp, PermissionSnapshot, PermissionStatus, Rect, RunningApp,
    ScreenshotMode, ScreenshotOptions, ScreenshotResult, WindowInfo, WindowKind, WindowSnapshot,
    WindowState,
};
