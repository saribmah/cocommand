//! Calendar cancel tool.
//!
//! This module provides the tool for deleting/canceling calendar events.

use serde_json::{json, Value};

use crate::applications::types::{Tool, ToolResult};

use super::script::{escape_applescript_string, run_applescript};

/// Tool ID for the cancel tool.
pub const TOOL_ID: &str = "calendar_cancel";

/// Tool for canceling/deleting a calendar event.
pub struct CalendarCancel;

impl Tool for CalendarCancel {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Cancel Event"
    }

    fn description(&self) -> &str {
        "Delete/cancel a calendar event from Apple Calendar. Searches for events whose summary/title contains the query and deletes the first match. Case-insensitive search."
    }

    fn schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query to find the event to cancel. Matches events whose summary/title contains this text (case-insensitive)."
                },
                "calendar": {
                    "type": "string",
                    "description": "Optional: Name of a specific calendar to search in. If not specified, searches all calendars."
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
            return ToolResult::error("Search query is required to find the event to cancel");
        }

        let calendar = inputs
            .get("calendar")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let escaped_query = escape_applescript_string(query);
        let script = build_cancel_script(&escaped_query, calendar);

        run_applescript(&script)
    }
}

fn build_cancel_script(query: &str, calendar: Option<&str>) -> String {
    let calendar_filter = match calendar {
        Some(c) => {
            let escaped = escape_applescript_string(c);
            format!(
                r#"
    set targetCals to {{}}
    repeat with cal in calendars
        if name of cal is "{}" then
            set end of targetCals to cal
        end if
    end repeat
    if (count of targetCals) = 0 then
        return "NOT_FOUND:Calendar '{}' not found"
    end if
"#,
                escaped, escaped
            )
        }
        None => r#"
    set targetCals to calendars
"#
        .to_string(),
    };

    // AppleScript to find and delete an event matching the query
    format!(
        r#"
tell application "Calendar"
    set queryLower to do shell script "echo " & quoted form of "{}" & " | tr '[:upper:]' '[:lower:]'"
    set foundEvent to missing value
    set foundName to ""
    set foundCal to missing value
{}
    -- Search through calendars for matching event
    repeat with cal in targetCals
        set calEvents to every event of cal
        repeat with evt in calEvents
            set evtSummary to summary of evt
            set summaryLower to do shell script "echo " & quoted form of evtSummary & " | tr '[:upper:]' '[:lower:]'"
            if summaryLower contains queryLower then
                set foundEvent to evt
                set foundName to evtSummary
                set foundCal to cal
                exit repeat
            end if
        end repeat
        if foundEvent is not missing value then exit repeat
    end repeat

    if foundEvent is missing value then
        return "NOT_FOUND:No event found matching '{}'"
    else
        delete foundEvent
        return "Event '" & foundName & "' has been deleted."
    end if
end tell
"#,
        query, calendar_filter, query
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = CalendarCancel;
        assert_eq!(tool.id(), "calendar_cancel");
    }

    #[test]
    fn test_tool_name() {
        let tool = CalendarCancel;
        assert_eq!(tool.name(), "Cancel Event");
    }

    #[test]
    fn test_tool_has_schema() {
        let tool = CalendarCancel;
        let schema = tool.schema();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
        assert!(schema["properties"]["calendar"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&json!("query")));
    }

    #[test]
    fn test_execute_empty_query() {
        let tool = CalendarCancel;
        let result = tool.execute(json!({}));
        assert_eq!(result.status, "error");
        assert!(result.message.contains("required"));
    }

    #[test]
    fn test_build_cancel_script() {
        let script = build_cancel_script("team meeting", None);
        assert!(script.contains("team meeting"));
        assert!(script.contains("delete foundEvent"));
        assert!(script.contains("set targetCals to calendars"));
    }

    #[test]
    fn test_build_cancel_script_with_calendar() {
        let script = build_cancel_script("team meeting", Some("Work"));
        assert!(script.contains("team meeting"));
        assert!(script.contains("name of cal is \"Work\""));
    }
}
