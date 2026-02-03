use std::collections::HashMap;

use objc2::{class, msg_send};
use objc2::rc::autoreleasepool;
use objc2::runtime::{AnyObject, Bool};
use serde::Serialize;

use crate::applescript::{applescript_escape, run_applescript};
use crate::util::nsstring_to_string;
use crate::window::{list_windows, WindowInfo};

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

pub fn list_open_apps(visible_only: bool) -> Result<Vec<RunningApp>, String> {
    autoreleasepool(|_| {
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
            results.retain(|app| app.windows.iter().any(|window| window.is_onscreen));
        }

        results.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(results)
    })
}

pub fn perform_app_action(
    bundle_id: Option<&str>,
    pid: Option<i32>,
    action: &str,
) -> Result<(), String> {
    const NS_APPLICATION_ACTIVATE_IGNORING_OTHER_APPS: u64 = 1;

    autoreleasepool(|_| {
        let workspace: *mut AnyObject = unsafe { msg_send![class!(NSWorkspace), sharedWorkspace] };
        if workspace.is_null() {
            return Err("failed to access NSWorkspace".to_string());
        }
        let running: *mut AnyObject = unsafe { msg_send![workspace, runningApplications] };
        if running.is_null() {
            return Err("failed to get running applications".to_string());
        }

        let count: usize = unsafe { msg_send![running, count] };
        let mut target: *mut AnyObject = std::ptr::null_mut();
        for index in 0..count {
            let app: *mut AnyObject = unsafe { msg_send![running, objectAtIndex: index] };
            if app.is_null() {
                continue;
            }
            if let Some(pid) = pid {
                let app_pid: i32 = unsafe { msg_send![app, processIdentifier] };
                if app_pid == pid {
                    target = app;
                    break;
                }
            }
            if let Some(bundle_id) = bundle_id {
                if let Some(app_bundle_id) =
                    nsstring_to_string(unsafe { msg_send![app, bundleIdentifier] })
                {
                    if app_bundle_id == bundle_id {
                        target = app;
                        break;
                    }
                }
            }
        }

        if target.is_null() {
            return Err("application not found".to_string());
        }

        match action {
            "activate" => {
                let success: Bool = unsafe {
                    msg_send![target, activateWithOptions: NS_APPLICATION_ACTIVATE_IGNORING_OTHER_APPS]
                };
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
                let success: Bool = unsafe { msg_send![target, hide] };
                if bool::from(success) {
                    Ok(())
                } else {
                    Err("failed to hide application".to_string())
                }
            }
            "quit" => {
                let success: Bool = unsafe { msg_send![target, terminate] };
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
    })
}
