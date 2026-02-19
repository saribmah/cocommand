use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum ClipboardItem {
    Text(String),
    Image(Vec<u8>),
    Files(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionStatus {
    pub id: String,
    pub label: String,
    pub granted: bool,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSnapshot {
    pub platform: String,
    pub permissions: Vec<PermissionStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowState {
    Onscreen,
    Offscreen,
    Minimized,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowKind {
    Normal,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub window_id: u32,
    pub title: Option<String>,
    pub bounds: Rect,
    pub layer: i32,
    pub is_onscreen: bool,
    pub minimized: Option<bool>,
    pub state: WindowState,
    pub kind: WindowKind,
    pub owner_pid: i32,
    pub owner_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSnapshot {
    pub snapshot_id: u64,
    pub windows: Vec<WindowInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningApp {
    pub name: String,
    pub bundle_id: Option<String>,
    pub pid: i32,
    pub is_active: bool,
    pub is_hidden: bool,
    pub has_windows: bool,
    pub windows: Vec<WindowInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledApp {
    pub name: String,
    pub bundle_id: Option<String>,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScreenshotMode {
    Interactive,
    Screen,
    Window,
    Rect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotOptions {
    pub mode: ScreenshotMode,
    pub display: Option<u32>,
    pub window_id: Option<u32>,
    pub rect: Option<String>,
    pub format: Option<String>,
    pub delay_seconds: Option<u64>,
    pub to_clipboard: bool,
    pub include_cursor: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotResult {
    pub path: Option<String>,
    pub filename: Option<String>,
    pub format: String,
    pub clipboard: bool,
}
