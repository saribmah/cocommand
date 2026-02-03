use super::{WindowInfo, WindowSnapshot};
use crate::permissions::check_accessibility;
use crate::window::{fallback, mapping, registry};
use crate::window::ax;

pub fn perform_window_action(
    window_id: u32,
    action: &str,
    snapshot_id: Option<u64>,
) -> Result<(), String> {
    let snapshot = match snapshot_id {
        Some(id) => registry::get_snapshot(id),
        None => None,
    };
    let windows = match snapshot {
        Some(WindowSnapshot { windows, .. }) => windows,
        None => super::list_windows(false)?,
    };
    let window = windows
        .into_iter()
        .find(|window| window.window_id == window_id)
        .ok_or_else(|| "window not found".to_string())?;

    if check_accessibility() {
        if let Ok(()) = perform_window_action_ax(&window, action) {
            return Ok(());
        }
    }

    fallback::perform_window_action_applescript(&window, action)
}

fn perform_window_action_ax(window: &WindowInfo, action: &str) -> Result<(), String> {
    let ax_windows = mapping::ax_windows_for_pid(window.owner_pid)?;
    let ax_window = mapping::match_window(window, &ax_windows)
        .ok_or_else(|| "failed to match AX window".to_string())?;

    let app = ax::create_application(window.owner_pid)?;

    match action {
        "focus" => {
            let _ = ax::set_frontmost(&app, true);
            ax::perform_raise(&ax_window.element)?;
        }
        "minimize" => {
            ax::set_minimized(&ax_window.element, true)?;
        }
        "close" => {
            if let Some(close_button) = ax::copy_close_button(&ax_window.element) {
                ax::perform_press(&close_button)?;
            } else {
                return Err("AX close button not available".to_string());
            }
        }
        _ => return Err(format!("unsupported window action {action}")),
    }

    Ok(())
}
