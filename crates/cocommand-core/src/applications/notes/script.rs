//! AppleScript execution utilities for Notes.
//!
//! This module provides the shared script execution logic used by all
//! Notes tools.

use std::process::Command;

use crate::applications::types::ToolResult;

/// Execute an AppleScript command to control Notes.
///
/// # Arguments
/// * `action` - The Notes action to perform (e.g., "activate")
/// * `success_message` - Message to return on success
///
/// # Returns
/// A `ToolResult` indicating success or failure with appropriate message.
pub fn run_notes_script(action: &str, success_message: &str) -> ToolResult {
    let script = format!("tell application \"Notes\" to {}", action);
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
pub fn run_applescript(script: &str) -> ToolResult {
    let output = Command::new("osascript").arg("-e").arg(script).output();

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout).trim().to_string();
            if stdout.starts_with("error:") {
                ToolResult::error(
                    stdout
                        .strip_prefix("error:")
                        .unwrap_or(&stdout)
                        .trim()
                        .to_string(),
                )
            } else if stdout.starts_with("NOT_FOUND:") {
                ToolResult::error(
                    stdout
                        .strip_prefix("NOT_FOUND:")
                        .unwrap_or(&stdout)
                        .trim()
                        .to_string(),
                )
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

/// Categorize content using simple keyword matching.
///
/// Returns a category string based on content analysis.
pub fn categorize_content(content: &str) -> String {
    let lower = content.to_lowercase();

    // Work-related keywords
    if lower.contains("meeting")
        || lower.contains("deadline")
        || lower.contains("project")
        || lower.contains("task")
        || lower.contains("work")
        || lower.contains("office")
        || lower.contains("client")
        || lower.contains("presentation")
        || lower.contains("report")
    {
        return "Work".to_string();
    }

    // Personal keywords
    if lower.contains("birthday")
        || lower.contains("family")
        || lower.contains("friend")
        || lower.contains("vacation")
        || lower.contains("holiday")
        || lower.contains("personal")
    {
        return "Personal".to_string();
    }

    // Shopping/errands keywords
    if lower.contains("buy")
        || lower.contains("shop")
        || lower.contains("grocery")
        || lower.contains("store")
        || lower.contains("order")
        || lower.contains("purchase")
    {
        return "Shopping".to_string();
    }

    // Ideas/creativity keywords
    if lower.contains("idea")
        || lower.contains("think")
        || lower.contains("maybe")
        || lower.contains("could")
        || lower.contains("should try")
        || lower.contains("brainstorm")
    {
        return "Ideas".to_string();
    }

    // Learning/education keywords
    if lower.contains("learn")
        || lower.contains("study")
        || lower.contains("read")
        || lower.contains("course")
        || lower.contains("tutorial")
        || lower.contains("book")
    {
        return "Learning".to_string();
    }

    // Health/fitness keywords
    if lower.contains("exercise")
        || lower.contains("workout")
        || lower.contains("health")
        || lower.contains("doctor")
        || lower.contains("medicine")
        || lower.contains("gym")
    {
        return "Health".to_string();
    }

    // Finance keywords
    if lower.contains("money")
        || lower.contains("budget")
        || lower.contains("pay")
        || lower.contains("bill")
        || lower.contains("invest")
        || lower.contains("expense")
    {
        return "Finance".to_string();
    }

    // Default category
    "General".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_applescript_string_simple() {
        assert_eq!(escape_applescript_string("hello"), "hello");
    }

    #[test]
    fn test_escape_applescript_string_quotes() {
        assert_eq!(
            escape_applescript_string("say \"hello\""),
            "say \\\"hello\\\""
        );
    }

    #[test]
    fn test_escape_applescript_string_backslash() {
        assert_eq!(
            escape_applescript_string("path\\to\\file"),
            "path\\\\to\\\\file"
        );
    }

    #[test]
    fn test_categorize_work() {
        assert_eq!(categorize_content("Meeting at 3pm"), "Work");
        assert_eq!(categorize_content("Project deadline tomorrow"), "Work");
    }

    #[test]
    fn test_categorize_personal() {
        assert_eq!(categorize_content("Mom's birthday next week"), "Personal");
        assert_eq!(categorize_content("Family dinner plans"), "Personal");
    }

    #[test]
    fn test_categorize_shopping() {
        assert_eq!(categorize_content("Buy milk and eggs"), "Shopping");
        assert_eq!(categorize_content("Order new shoes"), "Shopping");
    }

    #[test]
    fn test_categorize_ideas() {
        assert_eq!(categorize_content("Idea for new app"), "Ideas");
        assert_eq!(categorize_content("Maybe I should try this"), "Ideas");
    }

    #[test]
    fn test_categorize_general() {
        assert_eq!(categorize_content("Random thought here"), "General");
    }
}
