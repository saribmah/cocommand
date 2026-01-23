//! Reminders cancel tool.
//!
//! This module provides the tool for deleting/canceling reminders.

use serde_json::{json, Value};

use crate::applications::types::{Tool, ToolResult};

use super::script::{escape_applescript_string, run_applescript};

/// Tool ID for the cancel tool.
pub const TOOL_ID: &str = "reminders_cancel";

/// Tool for canceling/deleting a reminder.
pub struct RemindersCancel;

impl Tool for RemindersCancel {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Cancel Reminder"
    }

    fn description(&self) -> &str {
        "Delete/cancel a reminder from Apple Reminders. Searches for reminders whose name contains the query and deletes the first match. Case-insensitive search."
    }

    fn schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query to find the reminder to cancel. Matches reminders whose name contains this text (case-insensitive)."
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
            return ToolResult::error("Search query is required to find the reminder to cancel");
        }

        let list = inputs
            .get("list")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let escaped_query = escape_applescript_string(query);
        let script = build_cancel_script(&escaped_query, list);

        run_applescript(&script)
    }
}

fn build_cancel_script(query: &str, list: Option<&str>) -> String {
    let list_filter = match list {
        Some(l) => {
            let escaped = escape_applescript_string(l);
            format!("reminders in list \"{}\" whose completed is false", escaped)
        }
        None => "reminders whose completed is false".to_string(),
    };

    // AppleScript to find and delete a reminder matching the query
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
        delete foundReminder
        return "Reminder '" & foundName & "' has been deleted."
    end if
end tell
"#,
        query, list_filter, query
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = RemindersCancel;
        assert_eq!(tool.id(), "reminders_cancel");
    }

    #[test]
    fn test_tool_name() {
        let tool = RemindersCancel;
        assert_eq!(tool.name(), "Cancel Reminder");
    }

    #[test]
    fn test_tool_has_schema() {
        let tool = RemindersCancel;
        let schema = tool.schema();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
        assert!(schema["properties"]["list"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&json!("query")));
    }

    #[test]
    fn test_execute_empty_query() {
        let tool = RemindersCancel;
        let result = tool.execute(json!({}));
        assert_eq!(result.status, "error");
        assert!(result.message.contains("required"));
    }

    #[test]
    fn test_build_cancel_script() {
        let script = build_cancel_script("pay rent", None);
        assert!(script.contains("pay rent"));
        assert!(script.contains("delete foundReminder"));
        assert!(script.contains("reminders whose completed is false"));
    }

    #[test]
    fn test_build_cancel_script_with_list() {
        let script = build_cancel_script("pay rent", Some("Bills"));
        assert!(script.contains("pay rent"));
        assert!(script.contains("list \"Bills\""));
    }
}
