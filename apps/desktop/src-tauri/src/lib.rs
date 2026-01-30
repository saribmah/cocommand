use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

mod commands;
mod state;
mod window;
mod workspace_path;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load .env file from the crate root directory
    let _ = dotenvy::from_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env"));
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .try_init();

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
            let workspace_dir = workspace_path::load_workspace_dir(app.handle()).map_err(
                |error| std::io::Error::new(std::io::ErrorKind::Other, error),
            )?;
            let server_handle = tauri::async_runtime::block_on(
                state::start_server_with_retry(workspace_dir.clone(), 3, 200),
            )
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?;
            let app_state = state::AppState::new(workspace_dir.clone(), server_handle)
                .map_err(|error| {
                std::io::Error::new(std::io::ErrorKind::Other, error)
            })?;
            println!(
                "Backend server listening on {}",
                app_state.server_addr()
            );
            println!("Workspace directory: {}", app_state.workspace_dir().display());
            app.manage(app_state);

            let handle = app.handle();
            handle.global_shortcut().register("CmdOrCtrl+O")?;
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
            window::open_settings_window,
            window::hide_settings_window,
            commands::get_workspace_dir_cmd,
            commands::set_workspace_dir_cmd,
            commands::get_server_info_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
