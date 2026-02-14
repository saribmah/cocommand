use std::path::Path;
use std::process::{Command, ExitStatus};

use crate::error::{CoreError, CoreResult};

fn ensure_command_success(status: ExitStatus, command_label: &str) -> CoreResult<()> {
    if status.success() {
        Ok(())
    } else {
        Err(CoreError::Internal(format!(
            "{command_label} failed with status {status}"
        )))
    }
}

#[cfg(target_os = "macos")]
pub(super) fn open_path_native(path: &Path) -> CoreResult<()> {
    let status = Command::new("open")
        .arg(path)
        .status()
        .map_err(|error| CoreError::Internal(format!("failed to run open: {error}")))?;
    ensure_command_success(status, "open")
}

#[cfg(target_os = "linux")]
pub(super) fn open_path_native(path: &Path) -> CoreResult<()> {
    let status = Command::new("xdg-open")
        .arg(path)
        .status()
        .map_err(|error| CoreError::Internal(format!("failed to run xdg-open: {error}")))?;
    ensure_command_success(status, "xdg-open")
}

#[cfg(target_os = "windows")]
pub(super) fn open_path_native(path: &Path) -> CoreResult<()> {
    let status = Command::new("cmd")
        .arg("/C")
        .arg("start")
        .arg("")
        .arg(path)
        .status()
        .map_err(|error| CoreError::Internal(format!("failed to run start: {error}")))?;
    ensure_command_success(status, "start")
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub(super) fn open_path_native(_path: &Path) -> CoreResult<()> {
    Err(CoreError::Internal(
        "open_path is not supported on this platform".to_string(),
    ))
}

#[cfg(target_os = "macos")]
pub(super) fn reveal_path_native(path: &Path) -> CoreResult<()> {
    let status = Command::new("open")
        .arg("-R")
        .arg(path)
        .status()
        .map_err(|error| CoreError::Internal(format!("failed to reveal path: {error}")))?;
    ensure_command_success(status, "open -R")
}

#[cfg(not(target_os = "macos"))]
pub(super) fn reveal_path_native(path: &Path) -> CoreResult<()> {
    let parent = path.parent().unwrap_or(path);
    open_path_native(parent)
}
