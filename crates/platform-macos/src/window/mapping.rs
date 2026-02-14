use std::collections::HashMap;

use super::{rect_distance, Rect, WindowInfo};
use crate::window::ax::{self, AxElement};

#[derive(Debug, Clone)]
pub struct AxWindow {
    pub element: AxElement,
    pub title: Option<String>,
    pub bounds: Rect,
    pub minimized: Option<bool>,
}

pub fn enrich_windows_with_ax(windows: &mut [WindowInfo]) {
    let mut ax_cache: HashMap<i32, Vec<AxWindow>> = HashMap::new();

    for window in windows.iter_mut() {
        let ax_windows = ax_cache
            .entry(window.owner_pid)
            .or_insert_with(|| ax_windows_for_pid(window.owner_pid).unwrap_or_default());
        if let Some(ax_window) = match_window(window, ax_windows) {
            if window.title.is_none() {
                window.title = ax_window.title.clone();
            }
            window.minimized = ax_window.minimized;
            window.update_state();
        }
    }
}

pub fn ax_windows_for_pid(pid: i32) -> Result<Vec<AxWindow>, String> {
    let app = ax::create_application(pid)?;
    let window_elements = ax::copy_windows(&app)?;
    let mut windows = Vec::new();
    for element in window_elements {
        let bounds = match ax::copy_bounds(&element) {
            Some(bounds) => bounds,
            None => continue,
        };
        let title = ax::copy_title(&element);
        let minimized = ax::copy_minimized(&element);
        windows.push(AxWindow {
            element,
            title,
            bounds,
            minimized,
        });
    }
    Ok(windows)
}

pub fn match_window(window: &WindowInfo, ax_windows: &[AxWindow]) -> Option<AxWindow> {
    let mut best: Option<(usize, f64)> = None;
    for (index, ax_window) in ax_windows.iter().enumerate() {
        let distance = rect_distance(&window.bounds, &ax_window.bounds);
        if distance > 6.0 {
            continue;
        }
        if let Some((_, best_distance)) = best {
            if distance < best_distance {
                best = Some((index, distance));
            }
        } else {
            best = Some((index, distance));
        }
    }

    if let Some((index, _)) = best {
        return Some(ax_windows[index].clone());
    }

    if let Some(title) = window.title.as_ref() {
        let mut title_matches = ax_windows.iter().enumerate().filter(|(_, ax_window)| {
            ax_window
                .title
                .as_ref()
                .map(|ax_title| ax_title == title)
                .unwrap_or(false)
        });
        if let Some((index, _)) = title_matches.next() {
            if title_matches.next().is_none() {
                return Some(ax_windows[index].clone());
            }
        }
    }

    None
}
