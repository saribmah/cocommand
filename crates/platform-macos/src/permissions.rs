use std::process::Command;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
}

pub fn check_accessibility() -> bool {
    unsafe { AXIsProcessTrusted() }
}

pub fn check_screen_recording() -> bool {
    unsafe { CGPreflightScreenCaptureAccess() }
}

pub fn check_automation() -> Result<bool, String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to count processes")
        .output()
        .map_err(|error| format!("failed to run osascript: {error}"))?;

    if output.status.success() {
        return Ok(true);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    if stderr.contains("not authorized") || stderr.contains("not authorised") {
        return Ok(false);
    }
    Ok(false)
}

pub fn open_permission_settings(permission: &str) -> Result<(), String> {
    let url = match permission {
        "accessibility" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
        }
        "screen-recording" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
        }
        "automation" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Automation"
        }
        _ => return Err("unsupported permission".to_string()),
    };

    let status = Command::new("open")
        .arg(url)
        .status()
        .map_err(|error| format!("failed to open system settings: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("failed to open system settings (status {status})"))
    }
}
