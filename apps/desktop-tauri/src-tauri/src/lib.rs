use jsonschema::JSONSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
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

#[derive(Serialize, Deserialize, Clone)]
struct CommandDefinition {
    id: String,
    name: String,
    description: Option<String>,
    version: String,
    inputs: Option<Value>,
    steps: Vec<Value>,
    permissions: Option<Value>,
}

#[derive(Serialize)]
struct CommandLoadError {
    file: String,
    message: String,
}

#[derive(Serialize)]
struct CommandLoadResponse {
    commands: Vec<CommandDefinition>,
    errors: Vec<CommandLoadError>,
}

#[derive(Deserialize)]
struct CommandWriteRequest {
    command: CommandDefinition,
}

#[derive(Serialize)]
struct CommandWriteResponse {
    status: String,
    file: Option<String>,
    message: Option<String>,
}

#[derive(Deserialize)]
struct CommandDeleteRequest {
    id: String,
}

#[derive(Serialize)]
struct CommandDeleteResponse {
    status: String,
    file: Option<String>,
    message: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct WorkflowDefinition {
    id: String,
    name: String,
    description: Option<String>,
    version: String,
    inputs: Option<Value>,
    steps: Vec<Value>,
    permissions: Option<Value>,
}

#[derive(Serialize)]
struct WorkflowLoadError {
    file: String,
    message: String,
}

#[derive(Serialize)]
struct WorkflowLoadResponse {
    workflows: Vec<WorkflowDefinition>,
    errors: Vec<WorkflowLoadError>,
}

#[derive(Deserialize)]
struct WorkflowWriteRequest {
    workflow: WorkflowDefinition,
}

#[derive(Serialize)]
struct WorkflowWriteResponse {
    status: String,
    file: Option<String>,
    message: Option<String>,
}

#[derive(Deserialize)]
struct WorkflowDeleteRequest {
    id: String,
}

#[derive(Serialize)]
struct WorkflowDeleteResponse {
    status: String,
    file: Option<String>,
    message: Option<String>,
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

#[tauri::command]
fn list_commands(app: tauri::AppHandle) -> CommandLoadResponse {
    let schema = load_command_schema();
    let mut commands: Vec<CommandDefinition> = Vec::new();
    let mut errors: Vec<CommandLoadError> = Vec::new();

    for dir in command_dirs(&app) {
        let (dir_commands, dir_errors) = load_commands_from_dir(&dir, schema.as_ref());
        commands.extend(dir_commands);
        errors.extend(dir_errors);
    }

    commands.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    CommandLoadResponse { commands, errors }
}

#[tauri::command]
fn save_command(app: tauri::AppHandle, request: CommandWriteRequest) -> CommandWriteResponse {
    let schema = load_command_schema();
    if update_validation_error(&request.command, schema.as_ref()) {
        return CommandWriteResponse {
            status: "invalid".to_string(),
            file: None,
            message: Some("Command schema validation failed.".to_string()),
        };
    }

    let Some(dir) = user_commands_dir(&app) else {
        return CommandWriteResponse {
            status: "error".to_string(),
            file: None,
            message: Some("Unable to resolve commands directory.".to_string()),
        };
    };

    let file_path = dir.join(format!("{}.json", request.command.id));
    match serde_json::to_string_pretty(&request.command)
        .map_err(|error| error.to_string())
        .and_then(|data| fs::write(&file_path, data).map_err(|error| error.to_string()))
    {
        Ok(()) => CommandWriteResponse {
            status: "ok".to_string(),
            file: Some(file_path.display().to_string()),
            message: None,
        },
        Err(message) => CommandWriteResponse {
            status: "error".to_string(),
            file: Some(file_path.display().to_string()),
            message: Some(message),
        },
    }
}

#[tauri::command]
fn delete_command(app: tauri::AppHandle, request: CommandDeleteRequest) -> CommandDeleteResponse {
    let Some(dir) = user_commands_dir(&app) else {
        return CommandDeleteResponse {
            status: "error".to_string(),
            file: None,
            message: Some("Unable to resolve commands directory.".to_string()),
        };
    };

    let file_path = dir.join(format!("{}.json", request.id));
    match fs::remove_file(&file_path) {
        Ok(()) => CommandDeleteResponse {
            status: "ok".to_string(),
            file: Some(file_path.display().to_string()),
            message: None,
        },
        Err(error) => CommandDeleteResponse {
            status: "error".to_string(),
            file: Some(file_path.display().to_string()),
            message: Some(error.to_string()),
        },
    }
}

#[tauri::command]
fn list_workflows(app: tauri::AppHandle) -> WorkflowLoadResponse {
    let schema = load_workflow_schema();
    let mut workflows: Vec<WorkflowDefinition> = Vec::new();
    let mut errors: Vec<WorkflowLoadError> = Vec::new();

    for dir in workflow_dirs(&app) {
        let (dir_workflows, dir_errors) = load_workflows_from_dir(&dir, schema.as_ref());
        workflows.extend(dir_workflows);
        errors.extend(dir_errors);
    }

    workflows.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    WorkflowLoadResponse { workflows, errors }
}

#[tauri::command]
fn save_workflow(app: tauri::AppHandle, request: WorkflowWriteRequest) -> WorkflowWriteResponse {
    let schema = load_workflow_schema();
    if update_workflow_validation_error(&request.workflow, schema.as_ref()) {
        return WorkflowWriteResponse {
            status: "invalid".to_string(),
            file: None,
            message: Some("Workflow schema validation failed.".to_string()),
        };
    }

    let Some(dir) = user_workflows_dir(&app) else {
        return WorkflowWriteResponse {
            status: "error".to_string(),
            file: None,
            message: Some("Unable to resolve workflows directory.".to_string()),
        };
    };

    let file_path = dir.join(format!("{}.json", request.workflow.id));
    match serde_json::to_string_pretty(&request.workflow)
        .map_err(|error| error.to_string())
        .and_then(|data| fs::write(&file_path, data).map_err(|error| error.to_string()))
    {
        Ok(()) => WorkflowWriteResponse {
            status: "ok".to_string(),
            file: Some(file_path.display().to_string()),
            message: None,
        },
        Err(message) => WorkflowWriteResponse {
            status: "error".to_string(),
            file: Some(file_path.display().to_string()),
            message: Some(message),
        },
    }
}

#[tauri::command]
fn delete_workflow(app: tauri::AppHandle, request: WorkflowDeleteRequest) -> WorkflowDeleteResponse {
    let Some(dir) = user_workflows_dir(&app) else {
        return WorkflowDeleteResponse {
            status: "error".to_string(),
            file: None,
            message: Some("Unable to resolve workflows directory.".to_string()),
        };
    };

    let file_path = dir.join(format!("{}.json", request.id));
    match fs::remove_file(&file_path) {
        Ok(()) => WorkflowDeleteResponse {
            status: "ok".to_string(),
            file: Some(file_path.display().to_string()),
            message: None,
        },
        Err(error) => WorkflowDeleteResponse {
            status: "error".to_string(),
            file: Some(file_path.display().to_string()),
            message: Some(error.to_string()),
        },
    }
}

fn command_dirs(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(bundled_dir) = bundled_commands_dir() {
        dirs.push(bundled_dir);
    }

    if let Some(user_dir) = user_commands_dir(app) {
        dirs.push(user_dir);
    }

    dirs
}

fn workflow_dirs(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(bundled_dir) = bundled_workflows_dir() {
        dirs.push(bundled_dir);
    }

    if let Some(user_dir) = user_workflows_dir(app) {
        dirs.push(user_dir);
    }

    dirs
}

fn user_commands_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    if let Ok(app_data_dir) = app.path().app_data_dir() {
        let user_dir = app_data_dir.join("commands");
        let _ = fs::create_dir_all(&user_dir);
        return Some(user_dir);
    }
    None
}

fn user_workflows_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    if let Ok(app_data_dir) = app.path().app_data_dir() {
        let user_dir = app_data_dir.join("workflows");
        let _ = fs::create_dir_all(&user_dir);
        return Some(user_dir);
    }
    None
}

fn bundled_commands_dir() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.join("..").join("..").join("..");
    let bundled_dir = root_dir.join("packages").join("commands").join("examples");
    if bundled_dir.is_dir() {
        Some(bundled_dir)
    } else {
        None
    }
}

fn bundled_workflows_dir() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.join("..").join("..").join("..");
    let bundled_dir = root_dir.join("packages").join("workflows").join("examples");
    if bundled_dir.is_dir() {
        Some(bundled_dir)
    } else {
        None
    }
}

fn load_command_schema() -> Option<JSONSchema> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.join("..").join("..").join("..");
    let schema_path = root_dir
        .join("packages")
        .join("commands")
        .join("schema")
        .join("command.schema.json");
    let schema_raw = fs::read_to_string(schema_path).ok()?;
    let schema_json: Value = serde_json::from_str(&schema_raw).ok()?;
    JSONSchema::compile(&schema_json).ok()
}

fn load_workflow_schema() -> Option<JSONSchema> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.join("..").join("..").join("..");
    let schema_path = root_dir
        .join("packages")
        .join("workflows")
        .join("schema")
        .join("workflow.schema.json");
    let schema_raw = fs::read_to_string(schema_path).ok()?;
    let schema_json: Value = serde_json::from_str(&schema_raw).ok()?;
    JSONSchema::compile(&schema_json).ok()
}

fn update_validation_error(command: &CommandDefinition, schema: Option<&JSONSchema>) -> bool {
    let Some(schema) = schema else {
        return false;
    };

    let json = match serde_json::to_value(command) {
        Ok(json) => json,
        Err(_) => return true,
    };

    let invalid = schema.validate(&json).is_err();
    invalid
}

fn update_workflow_validation_error(
    workflow: &WorkflowDefinition,
    schema: Option<&JSONSchema>,
) -> bool {
    let Some(schema) = schema else {
        return false;
    };

    let json = match serde_json::to_value(workflow) {
        Ok(json) => json,
        Err(_) => return true,
    };

    let invalid = schema.validate(&json).is_err();
    invalid
}

fn load_commands_from_dir(
    directory: &Path,
    schema: Option<&JSONSchema>,
) -> (Vec<CommandDefinition>, Vec<CommandLoadError>) {
    let mut commands = Vec::new();
    let mut errors = Vec::new();

    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(CommandLoadError {
                file: directory.display().to_string(),
                message: error.to_string(),
            });
            return (commands, errors);
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(error) => {
                errors.push(CommandLoadError {
                    file: path.display().to_string(),
                    message: error.to_string(),
                });
                continue;
            }
        };

        let json: Value = match serde_json::from_str(&raw) {
            Ok(json) => json,
            Err(error) => {
                errors.push(CommandLoadError {
                    file: path.display().to_string(),
                    message: error.to_string(),
                });
                continue;
            }
        };

        if let Some(schema) = schema {
            if let Err(schema_errors) = schema.validate(&json) {
                let details = schema_errors
                    .map(|error| error.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                errors.push(CommandLoadError {
                    file: path.display().to_string(),
                    message: details,
                });
                continue;
            }
        }

        match serde_json::from_value::<CommandDefinition>(json) {
            Ok(command) => commands.push(command),
            Err(error) => errors.push(CommandLoadError {
                file: path.display().to_string(),
                message: error.to_string(),
            }),
        }
    }

    (commands, errors)
}

fn load_workflows_from_dir(
    directory: &Path,
    schema: Option<&JSONSchema>,
) -> (Vec<WorkflowDefinition>, Vec<WorkflowLoadError>) {
    let mut workflows = Vec::new();
    let mut errors = Vec::new();

    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(WorkflowLoadError {
                file: directory.display().to_string(),
                message: error.to_string(),
            });
            return (workflows, errors);
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(error) => {
                errors.push(WorkflowLoadError {
                    file: path.display().to_string(),
                    message: error.to_string(),
                });
                continue;
            }
        };

        let json: Value = match serde_json::from_str(&raw) {
            Ok(json) => json,
            Err(error) => {
                errors.push(WorkflowLoadError {
                    file: path.display().to_string(),
                    message: error.to_string(),
                });
                continue;
            }
        };

        if let Some(schema) = schema {
            if let Err(schema_errors) = schema.validate(&json) {
                let details = schema_errors
                    .map(|error| error.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                errors.push(WorkflowLoadError {
                    file: path.display().to_string(),
                    message: details,
                });
                continue;
            }
        }

        match serde_json::from_value::<WorkflowDefinition>(json) {
            Ok(workflow) => workflows.push(workflow),
            Err(error) => errors.push(WorkflowLoadError {
                file: path.display().to_string(),
                message: error.to_string(),
            }),
        }
    }

    (workflows, errors)
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
        .invoke_handler(tauri::generate_handler![
            execute_command,
            list_commands,
            save_command,
            delete_command,
            list_workflows,
            save_workflow,
            delete_workflow
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
