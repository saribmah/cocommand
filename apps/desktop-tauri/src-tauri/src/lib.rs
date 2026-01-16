use serde::{Deserialize, Serialize};
use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[derive(Deserialize)]
struct CommandRequest {
    input: String,
}

#[derive(Serialize)]
struct CommandResponse {
    status: String,
    output: String,
}

#[tauri::command]
fn execute_command(request: CommandRequest) -> CommandResponse {
    let trimmed = request.input.trim();
    if trimmed.is_empty() {
        return CommandResponse {
            status: "empty".to_string(),
            output: "Type a command to get started.".to_string(),
        };
    }

    CommandResponse {
        status: "ok".to_string(),
        output: format!("Command received: {}", trimmed),
    }
}

fn toggle_main_window(app: &tauri::AppHandle) {
    let window = app.get_webview_window("main");
    if let Some(window) = window {
        let is_visible = window.is_visible().unwrap_or(true);
        if is_visible {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_main_window(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let handle = app.handle();
            handle.global_shortcut().register("CmdOrCtrl+O")?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![execute_command])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
