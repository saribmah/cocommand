//! Reminders add tool.
//!
//! This module provides the tool for adding new reminders with optional due dates.

use serde_json::{json, Value};

use crate::applications::types::{Tool, ToolResult};

use super::script::{escape_applescript_string, run_applescript_with_message};

/// Tool ID for the add tool.
pub const TOOL_ID: &str = "reminders_add";

/// Tool for adding a new reminder.
pub struct RemindersAdd;

impl Tool for RemindersAdd {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Add Reminder"
    }

    fn description(&self) -> &str {
        "Add a new reminder to Apple Reminders. Use 'due_in_minutes' for relative time (e.g., 10 for '10 minutes from now') or 'due_at' for an ISO 8601 date/time string (e.g., '2024-01-15T13:00:00'). If neither is provided, creates a reminder without a due date."
    }

    fn schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "The title/name of the reminder (e.g., 'Pay rent', 'Call mom', 'Buy groceries')"
                },
                "due_in_minutes": {
                    "type": "integer",
                    "description": "Optional: Number of minutes from now when the reminder is due (e.g., 10 for '10 minutes from now', 60 for '1 hour from now')"
                },
                "due_at": {
                    "type": "string",
                    "description": "Optional: ISO 8601 date/time string for when the reminder is due (e.g., '2024-01-15T13:00:00'). The format must be YYYY-MM-DDTHH:MM:SS. Use due_in_minutes for relative times instead."
                },
                "list": {
                    "type": "string",
                    "description": "Optional: Name of the reminders list to add to (e.g., 'Work', 'Personal'). Uses default list if not specified."
                }
            },
            "required": ["title"]
        }))
    }

    fn execute(&self, inputs: Value) -> ToolResult {
        let title = inputs
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if title.is_empty() {
            return ToolResult::error("Reminder title is required");
        }

        let escaped_title = escape_applescript_string(title);

        let list = inputs
            .get("list")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let due_in_minutes = inputs.get("due_in_minutes").and_then(|v| v.as_i64());
        let due_at = inputs
            .get("due_at")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        // Build the AppleScript
        let script = if let Some(minutes) = due_in_minutes {
            // Use relative time from now
            let escaped_list = list.map(|l| escape_applescript_string(l));
            build_add_script_with_minutes(&escaped_title, minutes, escaped_list.as_deref())
        } else if let Some(date_str) = due_at {
            // Use specific date/time string
            let escaped_date = escape_applescript_string(date_str);
            let escaped_list = list.map(|l| escape_applescript_string(l));
            build_add_script_with_date(&escaped_title, &escaped_date, escaped_list.as_deref())
        } else {
            // No due date
            let escaped_list = list.map(|l| escape_applescript_string(l));
            build_add_script_no_due(&escaped_title, escaped_list.as_deref())
        };

        let success_msg = if due_in_minutes.is_some() {
            format!(
                "Reminder '{}' added, due in {} minute(s).",
                title,
                due_in_minutes.unwrap()
            )
        } else if let Some(date_str) = due_at {
            format!("Reminder '{}' added, due at {}.", title, date_str)
        } else {
            format!("Reminder '{}' added.", title)
        };

        run_applescript_with_message(&script, &success_msg)
    }
}

fn build_add_script_with_minutes(title: &str, minutes: i64, list: Option<&str>) -> String {
    let list_clause = match list {
        Some(l) => format!("list \"{}\"", l),
        None => "default list".to_string(),
    };

    format!(
        r#"
tell application "Reminders"
    set dueDate to (current date) + ({} * 60)
    tell {} to make new reminder with properties {{name:"{}", due date:dueDate}}
end tell
"#,
        minutes, list_clause, title
    )
}

fn build_add_script_with_date(title: &str, date_str: &str, list: Option<&str>) -> String {
    let list_clause = match list {
        Some(l) => format!("list \"{}\"", l),
        None => "default list".to_string(),
    };

    // Parse ISO 8601 format and construct AppleScript date explicitly
    // Expected format: YYYY-MM-DDTHH:MM:SS
    format!(
        r#"
tell application "Reminders"
    set isoString to "{}"

    -- Parse ISO 8601 format: YYYY-MM-DDTHH:MM:SS
    set yearStr to text 1 thru 4 of isoString
    set monthStr to text 6 thru 7 of isoString
    set dayStr to text 9 thru 10 of isoString
    set hourStr to text 12 thru 13 of isoString
    set minuteStr to text 15 thru 16 of isoString
    set secondStr to text 18 thru 19 of isoString

    -- Create date object
    set dueDate to current date
    set year of dueDate to (yearStr as integer)
    set month of dueDate to (monthStr as integer)
    set day of dueDate to (dayStr as integer)
    set hours of dueDate to (hourStr as integer)
    set minutes of dueDate to (minuteStr as integer)
    set seconds of dueDate to (secondStr as integer)

    tell {} to make new reminder with properties {{name:"{}", due date:dueDate}}
end tell
"#,
        date_str, list_clause, title
    )
}

fn build_add_script_no_due(title: &str, list: Option<&str>) -> String {
    let list_clause = match list {
        Some(l) => format!("list \"{}\"", l),
        None => "default list".to_string(),
    };

    format!(
        r#"
tell application "Reminders"
    tell {} to make new reminder with properties {{name:"{}"}}
end tell
"#,
        list_clause, title
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = RemindersAdd;
        assert_eq!(tool.id(), "reminders_add");
    }

    #[test]
    fn test_tool_name() {
        let tool = RemindersAdd;
        assert_eq!(tool.name(), "Add Reminder");
    }

    #[test]
    fn test_tool_has_schema() {
        let tool = RemindersAdd;
        let schema = tool.schema();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["title"].is_object());
        assert!(schema["properties"]["due_in_minutes"].is_object());
        assert!(schema["properties"]["due_at"].is_object());
        assert!(schema["properties"]["list"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&json!("title")));
    }

    #[test]
    fn test_execute_empty_title() {
        let tool = RemindersAdd;
        let result = tool.execute(json!({}));
        assert_eq!(result.status, "error");
        assert!(result.message.contains("required"));
    }

    #[test]
    fn test_build_script_with_minutes() {
        let script = build_add_script_with_minutes("Test reminder", 10, None);
        assert!(script.contains("Test reminder"));
        assert!(script.contains("10 * 60"));
        assert!(script.contains("default list"));
    }

    #[test]
    fn test_build_script_with_date() {
        let script = build_add_script_with_date("Test reminder", "2024-01-15T13:00:00", None);
        assert!(script.contains("Test reminder"));
        assert!(script.contains("2024-01-15T13:00:00"));
        assert!(script.contains("default list"));
        assert!(script.contains("yearStr"));
        assert!(script.contains("monthStr"));
    }

    #[test]
    fn test_build_script_with_custom_list() {
        let script = build_add_script_with_minutes("Test reminder", 10, Some("Work"));
        assert!(script.contains("Test reminder"));
        assert!(script.contains("list \"Work\""));
    }

    #[test]
    fn test_build_script_no_due() {
        let script = build_add_script_no_due("Test reminder", None);
        assert!(script.contains("Test reminder"));
        assert!(script.contains("default list"));
        assert!(!script.contains("due date"));
    }
}
