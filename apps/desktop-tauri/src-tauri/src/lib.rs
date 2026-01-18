use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

mod server;
mod applications;
mod commands;
mod llm;
mod tauri_commands;
mod window;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
            let handle = app.handle();
            handle.global_shortcut().register("CmdOrCtrl+O")?;
            tauri::async_runtime::spawn(async move {
                if let Ok(addr) = server::start().await {
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
            tauri_commands::plan_command,
            tauri_commands::run_workflow,
            window::hide_window,
            tauri_commands::list_commands,
            tauri_commands::save_command,
            tauri_commands::delete_command,
            tauri_commands::list_workflows,
            tauri_commands::save_workflow,
            tauri_commands::delete_workflow
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
