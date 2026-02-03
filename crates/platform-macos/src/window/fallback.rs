use crate::applescript::{applescript_escape, run_applescript};
use crate::window::WindowInfo;

pub fn perform_window_action_applescript(
    window: &WindowInfo,
    action: &str,
) -> Result<(), String> {
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
