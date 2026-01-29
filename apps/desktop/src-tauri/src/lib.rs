use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

mod commands;
mod state;
mod window;
#[cfg(test)]
mod e2e;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load .env file from the crate root directory
    let _ = dotenvy::from_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env"));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        window::toggle_main_window(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let workspace_dir = state::resolve_workspace_dir(app.handle()).map_err(|error| {
                std::io::Error::new(std::io::ErrorKind::Other, error)
            })?;
            let app_state = state::AppState::new(workspace_dir.clone()).map_err(|error| {
                std::io::Error::new(std::io::ErrorKind::Other, error)
            })?;
            app.manage(app_state);

            let handle = app.handle();
            handle.global_shortcut().register("CmdOrCtrl+O")?;
            tauri::async_runtime::spawn(async move {
                if let Ok(addr) = cocommand::server::start(workspace_dir).await {
                    println!("Backend server listening on {}", addr);
                }
            });
            if let Some(window) = app.get_webview_window("main") {
                let window_handle = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::Focused(false) = event {
                        let _ = window_handle.hide();
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            window::hide_window,
            commands::submit_command,
            commands::confirm_action,
            commands::get_workspace_snapshot,
            commands::get_recent_actions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
