use serde::Serialize;

use crate::applescript::run_applescript;

#[derive(Debug, Clone, Serialize)]
pub struct InstalledApp {
    pub name: String,
    pub bundle_id: Option<String>,
    pub path: String,
}

pub fn list_installed_apps() -> Vec<InstalledApp> {
    let mut apps = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for dir in app_directories() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("app") {
                    continue;
                }
                if let Some(app) = read_app_info(&path) {
                    let key = app.bundle_id.clone().unwrap_or_else(|| app.path.clone());
                    if seen.insert(key) {
                        apps.push(app);
                    }
                }
            }
        }
    }

    apps
}

pub fn open_installed_app(bundle_id: Option<&str>, path: &str) -> Result<(), String> {
    let mut command = std::process::Command::new("open");
    if let Some(id) = bundle_id {
        command.arg("-b").arg(id);
    } else {
        command.arg("-a").arg(path);
    }
    let output = command
        .output()
        .map_err(|error| format!("failed to execute open: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn installed_app_tools() -> Vec<(String, String, Option<String>, serde_json::Value)> {
    vec![
        (
            "open".to_string(),
            "Open".to_string(),
            Some("Open this application".to_string()),
            serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        ),
        (
            "applescript".to_string(),
            "AppleScript".to_string(),
            Some("Run AppleScript against this application".to_string()),
            serde_json::json!({
                "type": "object",
                "properties": { "script": { "type": "string" } },
                "required": ["script"]
            }),
        ),
    ]
}

pub fn execute_installed_app_tool(
    action_id: &str,
    input: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    match action_id {
        "applescript" => {
            let script = input
                .get("script")
                .and_then(|value| value.as_str())
                .ok_or_else(|| "missing applescript input".to_string())?;
            let output = run_applescript(script)?;
            Ok(serde_json::json!({ "output": output }))
        }
        _ => Err(format!("unsupported installed app action {action_id}")),
    }
}

fn app_directories() -> Vec<std::path::PathBuf> {
    let mut dirs = vec![
        std::path::PathBuf::from("/Applications"),
        std::path::PathBuf::from("/System/Applications"),
        std::path::PathBuf::from("/System/Applications/Utilities"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(std::path::PathBuf::from(home).join("Applications"));
    }
    dirs
}

fn read_app_info(app_path: &std::path::Path) -> Option<InstalledApp> {
    let plist_path = app_path.join("Contents").join("Info.plist");
    let plist = plist::Value::from_file(&plist_path).ok()?;
    let dict = plist.as_dictionary()?;

    let bundle_id = dict
        .get("CFBundleIdentifier")
        .and_then(|value| value.as_string())
        .map(|value| value.to_string());

    let name = dict
        .get("CFBundleDisplayName")
        .and_then(|value| value.as_string())
        .or_else(|| dict.get("CFBundleName").and_then(|value| value.as_string()))
        .map(|value| value.to_string())
        .unwrap_or_else(|| {
            app_path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("Unknown")
                .to_string()
        });

    Some(InstalledApp {
        name,
        bundle_id,
        path: app_path.to_string_lossy().to_string(),
    })
}
