use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalRect, Position};

#[tauri::command]
pub fn hide_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn toggle_main_window(app: &tauri::AppHandle) {
    let app_handle = app.clone();
    let app_handle_main = app.clone();
    let should_reposition = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let should_reposition_handle = should_reposition.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = app_handle_main.get_webview_window("main") {
            let is_visible = window.is_visible().unwrap_or(true);
            if is_visible {
                let _ = window.hide();
            } else {
                let _ = window.show();
                let _ = window.set_focus();
                should_reposition_handle.store(true, std::sync::atomic::Ordering::Relaxed);
                let _ = position_main_window_on_active_screen(&app_handle_main);
            }
        }
    });

    if should_reposition.load(std::sync::atomic::Ordering::Relaxed) {
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(40)).await;
            let app_handle_for_main = app_handle.clone();
            let _ = app_handle.run_on_main_thread(move || {
                let _ = position_main_window_on_active_screen(&app_handle_for_main);
            });
        });
    }
}

#[tauri::command]
pub fn open_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }

    let url = tauri::WebviewUrl::App("settings".into());
    let window = tauri::WebviewWindowBuilder::new(&app, "settings", url)
        .title("Settings")
        .inner_size(720.0, 520.0)
        .resizable(true)
        .decorations(false)
        .always_on_top(false)
        .build()
        .map_err(|error| error.to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn hide_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window.hide().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn position_window_on_active_screen(window: &tauri::WebviewWindow) -> Result<(), String> {
    let app = window.app_handle();
    let monitor = select_monitor_for_cursor(&app, window);
    let Some(monitor) = monitor else {
        return Ok(());
    };

    let work_area = monitor.work_area().clone();
    let window_size = window.outer_size().map_err(|error| error.to_string())?;

    let margin = 20i32;
    let available_width = (work_area.size.width as i32 - margin * 2).max(0);
    let available_height = (work_area.size.height as i32 - margin * 2).max(0);

    let mut x = work_area.position.x + margin + (available_width - window_size.width as i32) / 2;
    let mut y =
        work_area.position.y + margin + (available_height - window_size.height as i32) / 2;

    let min_x = work_area.position.x + margin;
    let min_y = work_area.position.y + margin;
    let mut max_x =
        work_area.position.x + work_area.size.width as i32 - window_size.width as i32 - margin;
    let mut max_y =
        work_area.position.y + work_area.size.height as i32 - window_size.height as i32 - margin;
    if max_x < min_x {
        max_x = min_x;
    }
    if max_y < min_y {
        max_y = min_y;
    }

    if x < min_x {
        x = min_x;
    }
    if y < min_y {
        y = min_y;
    }
    if x > max_x {
        x = max_x;
    }
    if y > max_y {
        y = max_y;
    }

    window
        .set_position(Position::Physical(PhysicalPosition::new(x, y)))
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn position_main_window_on_active_screen(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        position_window_on_active_screen(&window)?;
    }
    Ok(())
}

fn select_monitor_for_cursor(
    app: &AppHandle,
    window: &tauri::WebviewWindow,
) -> Option<tauri::Monitor> {
    if let Ok(cursor) = app.cursor_position() {
        if let Ok(monitors) = app.available_monitors() {
            for monitor in monitors {
                if point_in_rect(cursor, monitor.work_area()) {
                    return Some(monitor);
                }
            }
        }
    }

    window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())
}

fn point_in_rect(point: PhysicalPosition<f64>, rect: &PhysicalRect<i32, u32>) -> bool {
    let left = rect.position.x as f64;
    let right = (rect.position.x + rect.size.width as i32) as f64;
    let bottom = rect.position.y as f64;
    let top = (rect.position.y + rect.size.height as i32) as f64;
    point.x >= left && point.x <= right && point.y >= bottom && point.y <= top
}
