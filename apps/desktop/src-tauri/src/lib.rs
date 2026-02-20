use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

mod commands;
mod state;
mod window;
mod workspace_path;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load .env file from the crate root directory
    let _ = dotenvy::from_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env"));
    let builder = tracing_subscriber::fmt().with_span_events(FmtSpan::CLOSE);
    if let Ok(filter) = EnvFilter::try_from_default_env() {
        builder.with_env_filter(filter).init();
    } else {
        builder.with_env_filter(EnvFilter::new("info")).init();
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
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
            let workspace_dir = workspace_path::load_workspace_dir(app.handle())
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?;
            let app_state = state::AppState::new(workspace_dir.clone())
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?;
            info!(
                "Workspace directory: {}",
                app_state.workspace_dir().display()
            );
            app.manage(app_state);

            let handle = app.handle().clone();
            let workspace_clone = workspace_dir.clone();
            let handle_for_task = handle.clone();
            tauri::async_runtime::spawn(async move {
                match state::start_server_with_retry(workspace_clone, 3, 200).await {
                    Ok(server_handle) => {
                        if let Some(state) = handle_for_task.try_state::<state::AppState>() {
                            let _ = state.set_server(server_handle);
                            let _ = state.set_boot_status(state::BootStatus::Ready, None);
                            if let Some(addr) = state.server_addr() {
                                info!("Backend server listening on {}", addr);
                            }
                        }
                    }
                    Err(error) => {
                        if let Some(state) = handle_for_task.try_state::<state::AppState>() {
                            let _ = state.set_boot_status(state::BootStatus::Error, Some(error));
                        }
                    }
                }
            });

            handle.global_shortcut().register("CmdOrCtrl+O")?;
            if let Some(window) = app.get_webview_window("main") {
                let handle = app.handle();
                let handle_for_main = handle.clone();
                let _ = handle.run_on_main_thread(move || {
                    let _ = window::position_main_window_on_active_screen(&handle_for_main);
                });
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
            window::open_extension_window,
            window::close_extension_window,
            commands::get_workspace_dir_cmd,
            commands::set_workspace_dir_cmd,
            commands::get_server_info_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
