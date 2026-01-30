#[derive(Debug, Clone)]
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
                    let key = app
                        .bundle_id
                        .clone()
                        .unwrap_or_else(|| app.path.clone());
                    if seen.insert(key) {
                        apps.push(app);
                    }
                }
            }
        }
    }

    apps
}

pub fn open_app_by_bundle_id(_bundle_id: &str) -> Result<(), String> {
    Err("platform-macos not implemented".to_string())
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
