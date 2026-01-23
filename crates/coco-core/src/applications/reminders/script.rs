//! AppleScript execution utilities for Reminders.
//!
//! This module provides the shared script execution logic used by all
//! Reminders tools.

use std::process::Command;

use crate::applications::types::ToolResult;

/// Execute an AppleScript command to control Reminders.
///
/// # Arguments
/// * `action` - The Reminders action to perform (e.g., "activate")
/// * `success_message` - Message to return on success
///
/// # Returns
/// A `ToolResult` indicating success or failure with appropriate message.
pub fn run_reminders_script(action: &str, success_message: &str) -> ToolResult {
    let script = format!("tell application \"Reminders\" to {}", action);
    let output = Command::new("osascript").arg("-e").arg(script).output();

    match output {
        Ok(result) if result.status.success() => ToolResult::ok(success_message),
        Ok(result) => ToolResult::error(String::from_utf8_lossy(&result.stderr).to_string()),
        Err(error) => ToolResult::error(error.to_string()),
    }
}

/// Execute a multi-line AppleScript and return the result.
///
/// # Arguments
/// * `script` - The full AppleScript to execute
///
/// # Returns
/// A `ToolResult` with the script output or error.
/// Note: If the script output starts with "error:", it will be treated as an error.
pub fn run_applescript(script: &str) -> ToolResult {
    let output = Command::new("osascript").arg("-e").arg(script).output();

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout).trim().to_string();
            // Check if the script returned an error message (used for "not found" cases)
            if stdout.starts_with("error:") {
                ToolResult::error(stdout.strip_prefix("error:").unwrap_or(&stdout).trim().to_string())
            } else if stdout.starts_with("NOT_FOUND:") {
                ToolResult::error(stdout.strip_prefix("NOT_FOUND:").unwrap_or(&stdout).trim().to_string())
            } else {
                ToolResult::ok(stdout)
            }
        }
        Ok(result) => ToolResult::error(String::from_utf8_lossy(&result.stderr).to_string()),
        Err(error) => ToolResult::error(error.to_string()),
    }
}

/// Execute a multi-line AppleScript with a custom success message.
///
/// # Arguments
/// * `script` - The full AppleScript to execute
/// * `success_message` - Message to return on success (ignores stdout)
///
/// # Returns
/// A `ToolResult` with the custom message or error.
pub fn run_applescript_with_message(script: &str, success_message: &str) -> ToolResult {
    let output = Command::new("osascript").arg("-e").arg(script).output();

    match output {
        Ok(result) if result.status.success() => ToolResult::ok(success_message),
        Ok(result) => ToolResult::error(String::from_utf8_lossy(&result.stderr).to_string()),
        Err(error) => ToolResult::error(error.to_string()),
    }
}

/// Escape a string for use in AppleScript.
///
/// Escapes backslashes and double quotes to prevent injection.
pub fn escape_applescript_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_format() {
        let action = "activate";
        let script = format!("tell application \"Reminders\" to {}", action);
        assert!(script.contains("tell application"));
        assert!(script.contains("Reminders"));
        assert!(script.contains("activate"));
    }

    #[test]
    fn test_escape_applescript_string_simple() {
        assert_eq!(escape_applescript_string("hello"), "hello");
    }

    #[test]
    fn test_escape_applescript_string_quotes() {
        assert_eq!(escape_applescript_string("say \"hello\""), "say \\\"hello\\\"");
    }

    #[test]
    fn test_escape_applescript_string_backslash() {
        assert_eq!(escape_applescript_string("path\\to\\file"), "path\\\\to\\\\file");
    }

    #[test]
    fn test_escape_applescript_string_mixed() {
        assert_eq!(
            escape_applescript_string("He said \"hi\\there\""),
            "He said \\\"hi\\\\there\\\""
        );
    }
}
