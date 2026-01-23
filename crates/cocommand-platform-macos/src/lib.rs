#[derive(Debug, Clone)]
pub struct InstalledApp {
    pub name: String,
    pub bundle_id: Option<String>,
    pub path: String,
}

pub fn list_installed_apps() -> Vec<InstalledApp> {
    Vec::new()
}

pub fn open_app_by_bundle_id(_bundle_id: &str) -> Result<(), String> {
    Err("platform-macos not implemented".to_string())
}
