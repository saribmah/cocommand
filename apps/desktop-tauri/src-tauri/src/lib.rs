use serde::{Deserialize, Serialize};

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![execute_command])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
