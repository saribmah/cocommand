use serde::Serialize;

mod actions;
mod ax;
mod discovery;
mod fallback;
mod mapping;
mod registry;

#[derive(Debug, Clone, Serialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize)]
pub enum WindowState {
    Onscreen,
    Offscreen,
    Minimized,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub enum WindowKind {
    Normal,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
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

impl WindowInfo {
    fn update_state(&mut self) {
        self.state = match self.minimized {
            Some(true) => WindowState::Minimized,
            Some(false) => {
                if self.is_onscreen {
                    WindowState::Onscreen
                } else {
                    WindowState::Offscreen
                }
            }
            None => {
                if self.is_onscreen {
                    WindowState::Onscreen
                } else {
                    WindowState::Unknown
                }
            }
        };
    }
}

pub struct WindowSnapshot {
    pub snapshot_id: u64,
    pub windows: Vec<WindowInfo>,
}

pub fn list_windows(visible_only: bool) -> Result<Vec<WindowInfo>, String> {
    let mut windows = discovery::list_windows_cg(visible_only)?;
    if crate::permissions::check_accessibility() {
        mapping::enrich_windows_with_ax(&mut windows);
    }
    Ok(windows)
}

pub fn list_windows_snapshot(visible_only: bool) -> Result<WindowSnapshot, String> {
    let windows = list_windows(visible_only)?;
    registry::store_snapshot(windows)
}

pub fn get_windows_snapshot(snapshot_id: u64) -> Option<WindowSnapshot> {
    registry::get_snapshot(snapshot_id)
}

pub fn perform_window_action(
    window_id: u32,
    action: &str,
    snapshot_id: Option<u64>,
) -> Result<(), String> {
    actions::perform_window_action(window_id, action, snapshot_id)
}

fn classify_window_kind(layer: i32) -> WindowKind {
    if layer == 0 {
        WindowKind::Normal
    } else {
        WindowKind::Unknown
    }
}

fn rect_distance(a: &Rect, b: &Rect) -> f64 {
    let dx = (a.x - b.x).abs();
    let dy = (a.y - b.y).abs();
    let dw = (a.width - b.width).abs();
    let dh = (a.height - b.height).abs();
    dx + dy + dw + dh
}
