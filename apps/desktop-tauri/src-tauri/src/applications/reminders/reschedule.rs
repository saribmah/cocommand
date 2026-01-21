//! Reminders reschedule tool.
//!
//! This module provides the tool for rescheduling/updating reminder due dates.

use serde_json::{json, Value};

use crate::applications::types::{Tool, ToolResult};

use super::script::{escape_applescript_string, run_applescript};

/// Tool ID for the reschedule tool.
pub const TOOL_ID: &str = "reminders_reschedule";

/// Tool for rescheduling a reminder.
pub struct RemindersReschedule;

impl Tool for RemindersReschedule {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Reschedule Reminder"
    }

    fn description(&self) -> &str {
        "Update the due date of an existing reminder. Use 'new_due_in_minutes' for relative time (e.g., 10 for '10 minutes from now') or 'new_due_at' for an ISO 8601 date/time string (e.g., '2024-01-15T13:00:00'). Searches for reminders whose name contains the query."
    }

    fn schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query to find the reminder to reschedule. Matches reminders whose name contains this text (case-insensitive)."
                },
                "new_due_in_minutes": {
                    "type": "integer",
                    "description": "Optional: Number of minutes from now for the new due date (e.g., 10 for '10 minutes from now', 1440 for 'tomorrow at same time')."
                },
                "new_due_at": {
                    "type": "string",
                    "description": "Optional: ISO 8601 date/time string for the new due date (e.g., '2024-01-15T13:00:00'). The format must be YYYY-MM-DDTHH:MM:SS. Use new_due_in_minutes for relative times instead."
                },
                "list": {
                    "type": "string",
                    "description": "Optional: Name of a specific reminders list to search in. If not specified, searches all lists."
                }
            },
            "required": ["query"]
        }))
    }

    fn execute(&self, inputs: Value) -> ToolResult {
        let query = inputs
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if query.is_empty() {
            return ToolResult::error("Search query is required to find the reminder to reschedule");
        }

        let new_due_in_minutes = inputs.get("new_due_in_minutes").and_then(|v| v.as_i64());
        let new_due_at = inputs
            .get("new_due_at")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        if new_due_in_minutes.is_none() && new_due_at.is_none() {
            return ToolResult::error(
                "Either 'new_due_in_minutes' or 'new_due_at' is required to reschedule",
            );
        }

        let list = inputs
            .get("list")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let escaped_query = escape_applescript_string(query);
        let script = if let Some(minutes) = new_due_in_minutes {
            build_reschedule_script_minutes(&escaped_query, minutes, list)
        } else {
            let escaped_date = escape_applescript_string(new_due_at.unwrap());
            build_reschedule_script_date(&escaped_query, &escaped_date, list)
        };

        run_applescript(&script)
    }
}

fn build_reschedule_script_minutes(query: &str, minutes: i64, list: Option<&str>) -> String {
    let list_filter = match list {
        Some(l) => {
            let escaped = escape_applescript_string(l);
            format!("reminders in list \"{}\" whose completed is false", escaped)
        }
        None => "reminders whose completed is false".to_string(),
    };

    format!(
        r#"
tell application "Reminders"
    set queryLower to do shell script "echo " & quoted form of "{}" & " | tr '[:upper:]' '[:lower:]'"
    set foundReminder to missing value
    set foundName to ""

    repeat with r in ({})
        set reminderName to name of r
        set nameLower to do shell script "echo " & quoted form of reminderName & " | tr '[:upper:]' '[:lower:]'"
        if nameLower contains queryLower then
            set foundReminder to r
            set foundName to reminderName
            exit repeat
        end if
    end repeat

    if foundReminder is missing value then
        return "NOT_FOUND:No reminder found matching '{}'"
    else
        set newDueDate to (current date) + ({} * 60)
        set due date of foundReminder to newDueDate
        set dateString to (month of newDueDate as string) & " " & (day of newDueDate) & ", " & (year of newDueDate) & " at " & (time string of newDueDate)
        return "Reminder '" & foundName & "' rescheduled to " & dateString
    end if
end tell
"#,
        query, list_filter, query, minutes
    )
}

fn build_reschedule_script_date(query: &str, date_str: &str, list: Option<&str>) -> String {
    let list_filter = match list {
        Some(l) => {
            let escaped = escape_applescript_string(l);
            format!("reminders in list \"{}\" whose completed is false", escaped)
        }
        None => "reminders whose completed is false".to_string(),
    };

    // Parse ISO 8601 format and construct AppleScript date explicitly
    // Expected format: YYYY-MM-DDTHH:MM:SS
    format!(
        r#"
tell application "Reminders"
    set queryLower to do shell script "echo " & quoted form of "{}" & " | tr '[:upper:]' '[:lower:]'"
    set foundReminder to missing value
    set foundName to ""

    repeat with r in ({})
        set reminderName to name of r
        set nameLower to do shell script "echo " & quoted form of reminderName & " | tr '[:upper:]' '[:lower:]'"
        if nameLower contains queryLower then
            set foundReminder to r
            set foundName to reminderName
            exit repeat
        end if
    end repeat

    if foundReminder is missing value then
        return "NOT_FOUND:No reminder found matching '{}'"
    else
        set isoString to "{}"

        -- Parse ISO 8601 format: YYYY-MM-DDTHH:MM:SS
        set yearStr to text 1 thru 4 of isoString
        set monthStr to text 6 thru 7 of isoString
        set dayStr to text 9 thru 10 of isoString
        set hourStr to text 12 thru 13 of isoString
        set minuteStr to text 15 thru 16 of isoString
        set secondStr to text 18 thru 19 of isoString

        -- Create date object
        set newDueDate to current date
        set year of newDueDate to (yearStr as integer)
        set month of newDueDate to (monthStr as integer)
        set day of newDueDate to (dayStr as integer)
        set hours of newDueDate to (hourStr as integer)
        set minutes of newDueDate to (minuteStr as integer)
        set seconds of newDueDate to (secondStr as integer)

        set due date of foundReminder to newDueDate
        set dateString to (month of newDueDate as string) & " " & (day of newDueDate) & ", " & (year of newDueDate) & " at " & (time string of newDueDate)
        return "Reminder '" & foundName & "' rescheduled to " & dateString
    end if
end tell
"#,
        query, list_filter, query, date_str
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = RemindersReschedule;
        assert_eq!(tool.id(), "reminders_reschedule");
    }

    #[test]
    fn test_tool_name() {
        let tool = RemindersReschedule;
        assert_eq!(tool.name(), "Reschedule Reminder");
    }

    #[test]
    fn test_tool_has_schema() {
        let tool = RemindersReschedule;
        let schema = tool.schema();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
        assert!(schema["properties"]["new_due_in_minutes"].is_object());
        assert!(schema["properties"]["new_due_at"].is_object());
        assert!(schema["properties"]["list"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&json!("query")));
    }

    #[test]
    fn test_execute_empty_query() {
        let tool = RemindersReschedule;
        let result = tool.execute(json!({}));
        assert_eq!(result.status, "error");
        assert!(result.message.contains("required"));
    }

    #[test]
    fn test_execute_no_due_date() {
        let tool = RemindersReschedule;
        let result = tool.execute(json!({"query": "test"}));
        assert_eq!(result.status, "error");
        assert!(result.message.contains("new_due"));
    }

    #[test]
    fn test_build_reschedule_script_minutes() {
        let script = build_reschedule_script_minutes("pay rent", 60, None);
        assert!(script.contains("pay rent"));
        assert!(script.contains("60 * 60"));
        assert!(script.contains("set due date of foundReminder"));
    }

    #[test]
    fn test_build_reschedule_script_date() {
        let script = build_reschedule_script_date("pay rent", "2024-01-15T13:00:00", None);
        assert!(script.contains("pay rent"));
        assert!(script.contains("2024-01-15T13:00:00"));
        assert!(script.contains("yearStr"));
        assert!(script.contains("monthStr"));
    }

    #[test]
    fn test_build_reschedule_script_with_list() {
        let script = build_reschedule_script_minutes("pay rent", 60, Some("Bills"));
        assert!(script.contains("list \"Bills\""));
    }
}
