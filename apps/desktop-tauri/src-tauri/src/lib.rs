use jsonschema::JSONSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

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

#[derive(Deserialize)]
struct PlanRequest {
    input: String,
}

#[derive(Serialize)]
struct Intent {
    id: String,
    name: String,
    confidence: f32,
    parameters: Value,
}

#[derive(Serialize)]
struct PlanStep {
    id: String,
    tool: String,
    inputs: Value,
    status: String,
}

#[derive(Serialize)]
struct ExecutionPlan {
    id: String,
    intent: Intent,
    steps: Vec<PlanStep>,
    #[serde(rename = "createdAt")]
    created_at: String,
}

#[derive(Serialize)]
struct PlanResponse {
    status: String,
    plan: Option<ExecutionPlan>,
    message: Option<String>,
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

#[derive(Deserialize)]
struct WorkflowRunRequest {
    id: String,
}

#[derive(Serialize)]
struct WorkflowRunStep {
    id: String,
    command_id: String,
    status: String,
    message: Option<String>,
}

#[derive(Serialize)]
struct WorkflowRunResponse {
    status: String,
    summary: String,
    steps: Vec<WorkflowRunStep>,
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
fn plan_command(app: tauri::AppHandle, request: PlanRequest) -> PlanResponse {
    let trimmed = request.input.trim();
    if trimmed.is_empty() {
        return PlanResponse {
            status: "empty".to_string(),
            plan: None,
            message: Some("Type a command to get started.".to_string()),
        };
    }

    let query = normalize(trimmed);
    let command_schema = load_command_schema();
    let workflow_schema = load_workflow_schema();
    let mut commands = Vec::new();
    let mut workflows = Vec::new();

    for dir in command_dirs(&app) {
        let (dir_commands, _dir_errors) =
            load_commands_from_dir(&dir, command_schema.as_ref());
        commands.extend(dir_commands);
    }

    for dir in workflow_dirs(&app) {
        let (dir_workflows, _dir_errors) =
            load_workflows_from_dir(&dir, workflow_schema.as_ref());
        workflows.extend(dir_workflows);
    }

    let best_command = best_command_match(&commands, &query);
    let best_workflow = best_workflow_match(&workflows, &query);

    if let Some(best) = best_workflow {
        if best_command
            .as_ref()
            .map_or(true, |best_command| best.score >= best_command.score)
        {
            let intent =
                build_intent("workflow", &best.workflow.id, &best.workflow.name);
            return PlanResponse {
                status: "ok".to_string(),
                plan: Some(ExecutionPlan {
                    id: plan_id(trimmed),
                    intent,
                    steps: vec![PlanStep {
                        id: format!("step_{}", best.workflow.id),
                        tool: "workflow.run".to_string(),
                        inputs: json!({ "workflowId": best.workflow.id }),
                        status: "pending".to_string(),
                    }],
                    created_at: now_iso(),
                }),
                message: None,
            };
        }
    }

    if let Some(best) = best_command {
        let intent =
            build_intent("command", &best.command.id, &best.command.name);
        return PlanResponse {
            status: "ok".to_string(),
            plan: Some(ExecutionPlan {
                id: plan_id(trimmed),
                intent,
                steps: vec![PlanStep {
                    id: format!("step_{}", best.command.id),
                    tool: "command.run".to_string(),
                    inputs: json!({ "commandId": best.command.id }),
                    status: "pending".to_string(),
                }],
                created_at: now_iso(),
            }),
            message: None,
        };
    }

    let intent = Intent {
        id: format!("intent_{}", slug_id(trimmed)),
        name: "freeform".to_string(),
        confidence: 0.1,
        parameters: json!({ "text": trimmed }),
    };

    PlanResponse {
        status: "ok".to_string(),
        plan: Some(ExecutionPlan {
            id: plan_id(trimmed),
            intent,
            steps: vec![],
            created_at: now_iso(),
        }),
        message: None,
    }
}

#[tauri::command]
fn run_workflow(app: tauri::AppHandle, request: WorkflowRunRequest) -> WorkflowRunResponse {
    let workflow_schema = load_workflow_schema();
    let command_schema = load_command_schema();
    let mut workflows = Vec::new();
    let mut commands = Vec::new();

    for dir in workflow_dirs(&app) {
        let (dir_workflows, _dir_errors) =
            load_workflows_from_dir(&dir, workflow_schema.as_ref());
        workflows.extend(dir_workflows);
    }

    for dir in command_dirs(&app) {
        let (dir_commands, _dir_errors) =
            load_commands_from_dir(&dir, command_schema.as_ref());
        commands.extend(dir_commands);
    }

    let Some(workflow) = workflows.into_iter().find(|item| item.id == request.id) else {
        return WorkflowRunResponse {
            status: "error".to_string(),
            summary: format!("Workflow not found: {}", request.id),
            steps: Vec::new(),
        };
    };

    let mut step_reports = Vec::new();
    let mut status = "ok".to_string();
    let command_map: std::collections::HashMap<String, CommandDefinition> =
        commands.into_iter().map(|cmd| (cmd.id.clone(), cmd)).collect();

    for step_value in workflow.steps {
        let step: WorkflowStep = match serde_json::from_value(step_value) {
            Ok(step) => step,
            Err(error) => {
                step_reports.push(WorkflowRunStep {
                    id: "unknown".to_string(),
                    command_id: "unknown".to_string(),
                    status: "failed".to_string(),
                    message: Some(error.to_string()),
                });
                status = "failed".to_string();
                break;
            }
        };

        if !command_map.contains_key(&step.command_id) {
            step_reports.push(WorkflowRunStep {
                id: step.id.clone(),
                command_id: step.command_id.clone(),
                status: "failed".to_string(),
                message: Some("Command not found.".to_string()),
            });
            status = "failed".to_string();
            if step.on_error.as_deref().unwrap_or("halt") == "halt" {
                break;
            }
            continue;
        }

        step_reports.push(WorkflowRunStep {
            id: step.id,
            command_id: step.command_id,
            status: "completed".to_string(),
            message: None,
        });
    }

    let summary = if status == "ok" {
        format!("Workflow executed: {} step(s).", step_reports.len())
    } else {
        "Workflow execution failed.".to_string()
    };

    WorkflowRunResponse {
        status,
        summary,
        steps: step_reports,
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

#[tauri::command]
fn hide_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|error| error.to_string())?;
    }
    Ok(())
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

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

fn slug_id(value: &str) -> String {
    let normalized = normalize(value);
    let mut out = String::new();
    for ch in normalized.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if ch.is_whitespace() || ch == '-' || ch == '_' {
            out.push('_');
        }
    }
    if out.is_empty() {
        "request".to_string()
    } else {
        out
    }
}

fn plan_id(input: &str) -> String {
    format!("plan_{}_{}", slug_id(input), OffsetDateTime::now_utc().unix_timestamp_nanos())
}

fn now_iso() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string())
}

struct CommandMatch<'a> {
    command: &'a CommandDefinition,
    score: f32,
}

struct WorkflowMatch<'a> {
    workflow: &'a WorkflowDefinition,
    score: f32,
}

fn best_command_match<'a>(
    commands: &'a [CommandDefinition],
    query: &str,
) -> Option<CommandMatch<'a>> {
    let mut best: Option<CommandMatch<'a>> = None;
    for command in commands {
        let score = score_match(&command.name, command.description.as_deref(), query);
        if score <= 0.0 {
            continue;
        }
        if best.as_ref().map_or(true, |current| score > current.score) {
            best = Some(CommandMatch { command, score });
        }
    }
    best
}

fn best_workflow_match<'a>(
    workflows: &'a [WorkflowDefinition],
    query: &str,
) -> Option<WorkflowMatch<'a>> {
    let mut best: Option<WorkflowMatch<'a>> = None;
    for workflow in workflows {
        let score = score_match(&workflow.name, workflow.description.as_deref(), query);
        if score <= 0.0 {
            continue;
        }
        if best.as_ref().map_or(true, |current| score > current.score) {
            best = Some(WorkflowMatch { workflow, score });
        }
    }
    best
}

fn score_match(name: &str, description: Option<&str>, query: &str) -> f32 {
    if query.is_empty() {
        return 0.0;
    }
    let name_norm = normalize(name);
    let desc_norm = description.map(normalize).unwrap_or_default();

    if name_norm == query {
        return 1.0;
    }
    if name_norm.contains(query) {
        return 0.8;
    }
    if !desc_norm.is_empty() && desc_norm.contains(query) {
        return 0.5;
    }
    0.0
}

fn build_intent(kind: &str, item_id: &str, item_name: &str) -> Intent {
    Intent {
        id: format!("intent_{}", slug_id(item_name)),
        name: item_name.to_string(),
        confidence: 0.6,
        parameters: json!({
            "type": kind,
            "id": item_id,
        }),
    }
}

#[derive(Deserialize)]
struct WorkflowStep {
    id: String,
    #[serde(rename = "commandId")]
    command_id: String,
    inputs: Option<Value>,
    #[serde(rename = "onError")]
    on_error: Option<String>,
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
            execute_command,
            plan_command,
            run_workflow,
            hide_window,
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
