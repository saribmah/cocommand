use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::window::{
    kCGNullWindowID, kCGWindowIsOnscreen, kCGWindowLayer, kCGWindowListOptionAll,
    kCGWindowListOptionOnScreenOnly, kCGWindowName, kCGWindowNumber, kCGWindowOwnerName,
    kCGWindowOwnerPID, CGWindowListCopyWindowInfo,
};
use objc2::{class, msg_send};
use objc2::runtime::{AnyObject, Bool};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct InstalledApp {
    pub name: String,
    pub bundle_id: Option<String>,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindowInfo {
    pub window_id: u32,
    pub title: Option<String>,
    pub is_visible: bool,
    pub is_minimized: bool,
    pub owner_pid: i32,
    pub owner_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunningApp {
    pub name: String,
    pub bundle_id: Option<String>,
    pub pid: i32,
    pub is_active: bool,
    pub is_hidden: bool,
    pub has_windows: bool,
    pub windows: Vec<WindowInfo>,
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

pub fn list_open_apps(visible_only: bool) -> Result<Vec<RunningApp>, String> {
    let mut apps: HashMap<i32, RunningApp> = HashMap::new();

    let workspace: *mut AnyObject = unsafe { msg_send![class!(NSWorkspace), sharedWorkspace] };
    if workspace.is_null() {
        return Err("failed to access NSWorkspace".to_string());
    }
    let running: *mut AnyObject = unsafe { msg_send![workspace, runningApplications] };
    if running.is_null() {
        return Err("failed to get running applications".to_string());
    }

    let count: usize = unsafe { msg_send![running, count] };
    for index in 0..count {
        let app: *mut AnyObject = unsafe { msg_send![running, objectAtIndex: index] };
        if app.is_null() {
            continue;
        }
        let pid: i32 = unsafe { msg_send![app, processIdentifier] };
        let bundle_id = nsstring_to_string(unsafe { msg_send![app, bundleIdentifier] });
        let name = nsstring_to_string(unsafe { msg_send![app, localizedName] })
            .or_else(|| bundle_id.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        let is_active: Bool = unsafe { msg_send![app, isActive] };
        let is_hidden: Bool = unsafe { msg_send![app, isHidden] };

        apps.insert(
            pid,
            RunningApp {
                name,
                bundle_id,
                pid,
                is_active: bool::from(is_active),
                is_hidden: bool::from(is_hidden),
                has_windows: false,
                windows: Vec::new(),
            },
        );
    }

    let windows = list_windows(false)?;
    for window in windows {
        if window.owner_pid == 0 {
            continue;
        }
        let entry = apps.entry(window.owner_pid).or_insert_with(|| RunningApp {
            name: window
                .owner_name
                .clone()
                .unwrap_or_else(|| "Unknown".to_string()),
            bundle_id: None,
            pid: window.owner_pid,
            is_active: false,
            is_hidden: false,
            has_windows: false,
            windows: Vec::new(),
        });
        entry.windows.push(window);
    }

    let mut results: Vec<RunningApp> = apps
        .into_values()
        .map(|mut app| {
            app.has_windows = !app.windows.is_empty();
            app
        })
        .collect();

    if visible_only {
        results.retain(|app| app.windows.iter().any(|window| window.is_visible));
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(results)
}

pub fn open_app_by_bundle_id(_bundle_id: &str) -> Result<(), String> {
    Err("platform-macos not implemented".to_string())
}

pub fn perform_app_action(
    bundle_id: Option<&str>,
    pid: Option<i32>,
    action: &str,
) -> Result<(), String> {
    let app = running_application(bundle_id, pid)?;
    match action {
        "activate" => {
            let success: Bool = unsafe { msg_send![app, activateWithOptions: 1u64] };
            if bool::from(success) {
                return Ok(());
            }
            if let Some(bundle_id) = bundle_id {
                run_applescript(&format!(
                    "tell application id \"{}\" to activate",
                    applescript_escape(bundle_id)
                ))?;
                return Ok(());
            }
            Err("failed to activate application".to_string())
        }
        "hide" => {
            let success: Bool = unsafe { msg_send![app, hide] };
            if bool::from(success) {
                Ok(())
            } else {
                Err("failed to hide application".to_string())
            }
        }
        "quit" => {
            let success: Bool = unsafe { msg_send![app, terminate] };
            if bool::from(success) {
                return Ok(());
            }
            if let Some(bundle_id) = bundle_id {
                run_applescript(&format!(
                    "tell application id \"{}\" to quit",
                    applescript_escape(bundle_id)
                ))?;
                return Ok(());
            }
            Err("failed to quit application".to_string())
        }
        _ => Err(format!("unsupported app action {action}")),
    }
}

pub fn perform_window_action(window_id: u32, action: &str) -> Result<(), String> {
    let window = list_windows(false)?
        .into_iter()
        .find(|window| window.window_id == window_id)
        .ok_or_else(|| "window not found".to_string())?;

    let owner_name = window
        .owner_name
        .clone()
        .ok_or_else(|| "window has no owner name".to_string())?;
    let window_target = match window.title.as_ref() {
        Some(title) => format!("window \"{}\"", applescript_escape(title)),
        None => "window 1".to_string(),
    };

    let script = match action {
        "focus" => format!(
            "tell application \"System Events\"\n  tell application process \"{}\"\n    set frontmost to true\n    perform action \"AXRaise\" of {}\n  end tell\nend tell",
            applescript_escape(&owner_name),
            window_target
        ),
        "minimize" => format!(
            "tell application \"System Events\"\n  tell application process \"{}\"\n    set value of attribute \"AXMinimized\" of {} to true\n  end tell\nend tell",
            applescript_escape(&owner_name),
            window_target
        ),
        "close" => format!(
            "tell application \"System Events\"\n  tell application process \"{}\"\n    click (first button whose subrole is \"AXCloseButton\") of {}\n  end tell\nend tell",
            applescript_escape(&owner_name),
            window_target
        ),
        _ => return Err(format!("unsupported window action {action}")),
    };

    run_applescript(&script)?;
    Ok(())
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

pub fn run_applescript(script: &str) -> Result<String, String> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|error| format!("failed to execute osascript: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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

fn running_application(
    bundle_id: Option<&str>,
    pid: Option<i32>,
) -> Result<*mut AnyObject, String> {
    unsafe {
        if let Some(bundle_id) = bundle_id {
            let bundle_id = nsstring_from_str(bundle_id)?;
            let app: *mut AnyObject = msg_send![class!(NSRunningApplication), runningApplicationWithBundleIdentifier: bundle_id];
            if !app.is_null() {
                return Ok(app);
            }
        }
        if let Some(pid) = pid {
            let app: *mut AnyObject = msg_send![class!(NSRunningApplication), runningApplicationWithProcessIdentifier: pid];
            if !app.is_null() {
                return Ok(app);
            }
        }
    }
    Err("application not found".to_string())
}

fn list_windows(visible_only: bool) -> Result<Vec<WindowInfo>, String> {
    let options = if visible_only {
        kCGWindowListOptionOnScreenOnly
    } else {
        kCGWindowListOptionAll
    };
    let array_ref = unsafe { CGWindowListCopyWindowInfo(options, kCGNullWindowID) };
    if array_ref.is_null() {
        return Ok(Vec::new());
    }
    let array: CFArray<CFDictionary<CFType, CFType>> =
        unsafe { TCFType::wrap_under_create_rule(array_ref) };

    let mut windows = Vec::new();
    for dict in array.iter() {
        let window_id = unsafe { dict_get_i64(&dict, kCGWindowNumber) }.unwrap_or(0) as u32;
        let owner_pid = unsafe { dict_get_i64(&dict, kCGWindowOwnerPID) }.unwrap_or(0) as i32;
        let owner_name = unsafe { dict_get_string(&dict, kCGWindowOwnerName) };
        let title = unsafe { dict_get_string(&dict, kCGWindowName) };
        let is_visible = unsafe { dict_get_bool(&dict, kCGWindowIsOnscreen) }.unwrap_or(false);
        let layer = unsafe { dict_get_i64(&dict, kCGWindowLayer) }.unwrap_or(0);
        if layer != 0 {
            continue;
        }
        if window_id == 0 || owner_pid == 0 {
            continue;
        }

        windows.push(WindowInfo {
            window_id,
            title,
            is_visible,
            is_minimized: !is_visible,
            owner_pid,
            owner_name,
        });
    }

    Ok(windows)
}

fn dict_get_string(
    dict: &CFDictionary<CFType, CFType>,
    key: CFStringRef,
) -> Option<String> {
    let key = unsafe { CFString::wrap_under_get_rule(key) };
    let value = dict.find(key.as_CFType())?;
    value.downcast::<CFString>().map(|value| value.to_string())
}

fn dict_get_i64(
    dict: &CFDictionary<CFType, CFType>,
    key: CFStringRef,
) -> Option<i64> {
    let key = unsafe { CFString::wrap_under_get_rule(key) };
    let value = dict.find(key.as_CFType())?;
    value
        .downcast::<CFNumber>()
        .and_then(|value| value.to_i64())
}

fn dict_get_bool(
    dict: &CFDictionary<CFType, CFType>,
    key: CFStringRef,
) -> Option<bool> {
    let key = unsafe { CFString::wrap_under_get_rule(key) };
    let value = dict.find(key.as_CFType())?;
    value.downcast::<CFBoolean>().map(bool::from)
}

fn nsstring_from_str(value: &str) -> Result<*mut AnyObject, String> {
    let cstring = CString::new(value)
        .map_err(|error| format!("invalid string for NSString: {error}"))?;
    let nsstring: *mut AnyObject = unsafe {
        msg_send![class!(NSString), stringWithUTF8String: cstring.as_ptr()]
    };
    if nsstring.is_null() {
        return Err("failed to create NSString".to_string());
    }
    Ok(nsstring)
}

fn nsstring_to_string(value: *mut AnyObject) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let cstr: *const c_char = unsafe { msg_send![value, UTF8String] };
    if cstr.is_null() {
        return None;
    }
    let value = unsafe { CStr::from_ptr(cstr) };
    Some(value.to_string_lossy().to_string())
}

fn applescript_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\"', "\\\"")
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
