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
use serde::Serialize;

use crate::applescript::{applescript_escape, run_applescript};

#[derive(Debug, Clone, Serialize)]
pub struct WindowInfo {
    pub window_id: u32,
    pub title: Option<String>,
    pub is_visible: bool,
    pub is_minimized: bool,
    pub owner_pid: i32,
    pub owner_name: Option<String>,
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

pub fn list_windows(visible_only: bool) -> Result<Vec<WindowInfo>, String> {
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
