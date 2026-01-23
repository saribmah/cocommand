//! Reminders list_upcoming tool.
//!
//! This module provides the tool for listing upcoming/incomplete reminders.

use serde_json::{json, Value};

use crate::applications::types::{Tool, ToolResult};

use super::script::{escape_applescript_string, run_applescript};

/// Tool ID for the list_upcoming tool.
pub const TOOL_ID: &str = "reminders_list_upcoming";

/// Tool for listing upcoming incomplete reminders.
pub struct RemindersListUpcoming;

impl Tool for RemindersListUpcoming {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "List Upcoming"
    }

    fn description(&self) -> &str {
        "List incomplete reminders from Apple Reminders. Returns reminders with due dates sorted by due date (earliest first), followed by reminders without due dates. Use 'limit' to control how many reminders to return."
    }

    fn schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "limit": {
                    "type": "integer",
                    "description": "Optional: Maximum number of reminders to return. Defaults to 10."
                },
                "list": {
                    "type": "string",
                    "description": "Optional: Name of a specific reminders list to filter by (e.g., 'Work', 'Personal'). If not specified, returns reminders from all lists."
                },
                "include_no_due": {
                    "type": "boolean",
                    "description": "Optional: Whether to include reminders without a due date. Defaults to true."
                }
            }
        }))
    }

    fn execute(&self, inputs: Value) -> ToolResult {
        let limit = inputs
            .get("limit")
            .and_then(|v| v.as_i64())
            .unwrap_or(10) as usize;

        let list = inputs
            .get("list")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let include_no_due = inputs
            .get("include_no_due")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let script = build_list_script(limit, list, include_no_due);
        let result = run_applescript(&script);

        if result.status == "ok" {
            if result.message.trim().is_empty() {
                ToolResult::ok("No upcoming reminders found.")
            } else {
                result
            }
        } else {
            result
        }
    }
}

fn build_list_script(limit: usize, list: Option<&str>, include_no_due: bool) -> String {
    let list_filter = match list {
        Some(l) => {
            let escaped = escape_applescript_string(l);
            format!("reminders in list \"{}\" whose completed is false", escaped)
        }
        None => "reminders whose completed is false".to_string(),
    };

    // AppleScript to get reminders, sort by due date, and format them
    // We collect all matching reminders first, sort them, then apply the limit
    format!(
        r#"
tell application "Reminders"
    set withDueList to {{}}
    set noDueList to {{}}
    set reminderList to ({})

    -- Collect reminders into two lists: with due date and without
    repeat with r in reminderList
        set reminderName to name of r
        try
            set dueDate to due date of r
            -- Include all incomplete reminders with due dates (both past and future)
            set end of withDueList to {{reminderName, dueDate}}
        on error
            -- No due date
            if {} then
                set end of noDueList to reminderName
            end if
        end try
    end repeat

    -- Sort withDueList by due date (bubble sort - earliest first)
    set listCount to count of withDueList
    if listCount > 1 then
        repeat with i from 1 to listCount - 1
            repeat with j from 1 to listCount - i
                set item1 to item j of withDueList
                set item2 to item (j + 1) of withDueList
                set date1 to item 2 of item1
                set date2 to item 2 of item2
                if date1 > date2 then
                    set item j of withDueList to item2
                    set item (j + 1) of withDueList to item1
                end if
            end repeat
        end repeat
    end if

    -- Build output list with limit
    set outputList to {{}}
    set counter to 0

    -- Add sorted due-date reminders first (earliest first)
    repeat with reminderData in withDueList
        if counter >= {} then exit repeat
        set reminderName to item 1 of reminderData
        set dueDate to item 2 of reminderData
        set dateString to (month of dueDate as string) & " " & (day of dueDate) & ", " & (year of dueDate) & " at " & (time string of dueDate)
        set end of outputList to "- " & reminderName & " (due: " & dateString & ")"
        set counter to counter + 1
    end repeat

    -- Add no-due-date reminders after
    repeat with reminderName in noDueList
        if counter >= {} then exit repeat
        set end of outputList to "- " & reminderName & " (no due date)"
        set counter to counter + 1
    end repeat

    if (count of outputList) = 0 then
        return ""
    else
        set AppleScript's text item delimiters to linefeed
        return outputList as text
    end if
end tell
"#,
        list_filter, include_no_due, limit, limit
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = RemindersListUpcoming;
        assert_eq!(tool.id(), "reminders_list_upcoming");
    }

    #[test]
    fn test_tool_name() {
        let tool = RemindersListUpcoming;
        assert_eq!(tool.name(), "List Upcoming");
    }

    #[test]
    fn test_tool_has_schema() {
        let tool = RemindersListUpcoming;
        let schema = tool.schema();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["limit"].is_object());
        assert!(schema["properties"]["list"].is_object());
        assert!(schema["properties"]["include_no_due"].is_object());
    }

    #[test]
    fn test_build_list_script_defaults() {
        let script = build_list_script(10, None, true);
        assert!(script.contains("reminders whose completed is false"));
        assert!(script.contains(">= 10"));
        assert!(script.contains("true"));
        // Verify sorting is present
        assert!(script.contains("bubble sort"));
        assert!(script.contains("earliest first"));
    }

    #[test]
    fn test_build_list_script_with_list() {
        let script = build_list_script(5, Some("Work"), false);
        assert!(script.contains("list \"Work\""));
        assert!(script.contains("5"));
        assert!(script.contains("false"));
    }
}
