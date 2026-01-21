//! Calendar reschedule tool.
//!
//! This module provides the tool for rescheduling/updating calendar event times.

use serde_json::{json, Value};

use crate::applications::types::{Tool, ToolResult};

use super::script::{escape_applescript_string, run_applescript};

/// Tool ID for the reschedule tool.
pub const TOOL_ID: &str = "calendar_reschedule";

/// Tool for rescheduling a calendar event.
pub struct CalendarReschedule;

impl Tool for CalendarReschedule {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Reschedule Event"
    }

    fn description(&self) -> &str {
        "Update the start/end time of an existing calendar event. Use 'new_start_in_minutes' for relative time (e.g., 60 for '1 hour from now') or 'new_start_at' for an ISO 8601 date/time string (e.g., '2024-01-15T13:00:00'). Optionally specify 'new_end_at' or 'new_duration_minutes' for the new duration. Searches for events whose summary/title contains the query."
    }

    fn schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query to find the event to reschedule. Matches events whose summary/title contains this text (case-insensitive)."
                },
                "new_start_in_minutes": {
                    "type": "integer",
                    "description": "Optional: Number of minutes from now for the new start time (e.g., 60 for '1 hour from now', 1440 for 'tomorrow at same time')."
                },
                "new_start_at": {
                    "type": "string",
                    "description": "Optional: ISO 8601 date/time string for the new start time (e.g., '2024-01-15T13:00:00'). The format must be YYYY-MM-DDTHH:MM:SS. Use new_start_in_minutes for relative times instead."
                },
                "new_end_at": {
                    "type": "string",
                    "description": "Optional: ISO 8601 date/time string for the new end time (e.g., '2024-01-15T14:00:00'). The format must be YYYY-MM-DDTHH:MM:SS."
                },
                "new_duration_minutes": {
                    "type": "integer",
                    "description": "Optional: New duration of the event in minutes. If not specified, keeps the original duration."
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
            return ToolResult::error("Search query is required to find the event to reschedule");
        }

        let new_start_in_minutes = inputs.get("new_start_in_minutes").and_then(|v| v.as_i64());
        let new_start_at = inputs
            .get("new_start_at")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        if new_start_in_minutes.is_none() && new_start_at.is_none() {
            return ToolResult::error(
                "Either 'new_start_in_minutes' or 'new_start_at' is required to reschedule",
            );
        }

        let new_end_at = inputs
            .get("new_end_at")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let new_duration_minutes = inputs.get("new_duration_minutes").and_then(|v| v.as_i64());

        let calendar = inputs
            .get("calendar")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let escaped_query = escape_applescript_string(query);
        let script = if let Some(minutes) = new_start_in_minutes {
            build_reschedule_script_minutes(&escaped_query, minutes, new_duration_minutes, calendar)
        } else {
            let escaped_start = escape_applescript_string(new_start_at.unwrap());
            let escaped_end = new_end_at.map(|e| escape_applescript_string(e));
            build_reschedule_script_date(
                &escaped_query,
                &escaped_start,
                escaped_end.as_deref(),
                new_duration_minutes,
                calendar,
            )
        };

        run_applescript(&script)
    }
}

fn build_reschedule_script_minutes(
    query: &str,
    start_minutes: i64,
    duration_minutes: Option<i64>,
    calendar: Option<&str>,
) -> String {
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

    let duration_script = match duration_minutes {
        Some(mins) => format!("set newEndDate to newStartDate + ({} * 60)", mins),
        None => "set newEndDate to newStartDate + originalDuration".to_string(),
    };

    format!(
        r#"
tell application "Calendar"
    set queryLower to do shell script "echo " & quoted form of "{}" & " | tr '[:upper:]' '[:lower:]'"
    set foundEvent to missing value
    set foundName to ""
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
                exit repeat
            end if
        end repeat
        if foundEvent is not missing value then exit repeat
    end repeat

    if foundEvent is missing value then
        return "NOT_FOUND:No event found matching '{}'"
    else
        -- Calculate original duration
        set originalStart to start date of foundEvent
        set originalEnd to end date of foundEvent
        set originalDuration to originalEnd - originalStart

        -- Set new times
        set newStartDate to (current date) + ({} * 60)
        {}

        -- Update end date first, then start date to avoid "start must be before end" error
        set end date of foundEvent to newEndDate
        set start date of foundEvent to newStartDate

        set dateString to (month of newStartDate as string) & " " & (day of newStartDate) & ", " & (year of newStartDate) & " at " & (time string of newStartDate)
        return "Event '" & foundName & "' rescheduled to " & dateString
    end if
end tell
"#,
        query, calendar_filter, query, start_minutes, duration_script
    )
}

fn build_reschedule_script_date(
    query: &str,
    start_date_str: &str,
    end_date_str: Option<&str>,
    duration_minutes: Option<i64>,
    calendar: Option<&str>,
) -> String {
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

    let end_date_script = match end_date_str {
        Some(end_str) => format!(
            r#"
        -- Parse end date ISO 8601 format: YYYY-MM-DDTHH:MM:SS
        set endIsoString to "{}"
        set endYearStr to text 1 thru 4 of endIsoString
        set endMonthStr to text 6 thru 7 of endIsoString
        set endDayStr to text 9 thru 10 of endIsoString
        set endHourStr to text 12 thru 13 of endIsoString
        set endMinuteStr to text 15 thru 16 of endIsoString
        set endSecondStr to text 18 thru 19 of endIsoString

        set newEndDate to current date
        set year of newEndDate to (endYearStr as integer)
        set month of newEndDate to (endMonthStr as integer)
        set day of newEndDate to (endDayStr as integer)
        set hours of newEndDate to (endHourStr as integer)
        set minutes of newEndDate to (endMinuteStr as integer)
        set seconds of newEndDate to (endSecondStr as integer)
"#,
            end_str
        ),
        None => match duration_minutes {
            Some(mins) => format!("        set newEndDate to newStartDate + ({} * 60)", mins),
            None => "        set newEndDate to newStartDate + originalDuration".to_string(),
        },
    };

    format!(
        r#"
tell application "Calendar"
    set queryLower to do shell script "echo " & quoted form of "{}" & " | tr '[:upper:]' '[:lower:]'"
    set foundEvent to missing value
    set foundName to ""
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
                exit repeat
            end if
        end repeat
        if foundEvent is not missing value then exit repeat
    end repeat

    if foundEvent is missing value then
        return "NOT_FOUND:No event found matching '{}'"
    else
        -- Calculate original duration
        set originalStart to start date of foundEvent
        set originalEnd to end date of foundEvent
        set originalDuration to originalEnd - originalStart

        -- Parse start date ISO 8601 format: YYYY-MM-DDTHH:MM:SS
        set isoString to "{}"
        set yearStr to text 1 thru 4 of isoString
        set monthStr to text 6 thru 7 of isoString
        set dayStr to text 9 thru 10 of isoString
        set hourStr to text 12 thru 13 of isoString
        set minuteStr to text 15 thru 16 of isoString
        set secondStr to text 18 thru 19 of isoString

        set newStartDate to current date
        set year of newStartDate to (yearStr as integer)
        set month of newStartDate to (monthStr as integer)
        set day of newStartDate to (dayStr as integer)
        set hours of newStartDate to (hourStr as integer)
        set minutes of newStartDate to (minuteStr as integer)
        set seconds of newStartDate to (secondStr as integer)

{}

        -- Update end date first, then start date to avoid "start must be before end" error
        set end date of foundEvent to newEndDate
        set start date of foundEvent to newStartDate

        set dateString to (month of newStartDate as string) & " " & (day of newStartDate) & ", " & (year of newStartDate) & " at " & (time string of newStartDate)
        return "Event '" & foundName & "' rescheduled to " & dateString
    end if
end tell
"#,
        query, calendar_filter, query, start_date_str, end_date_script
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = CalendarReschedule;
        assert_eq!(tool.id(), "calendar_reschedule");
    }

    #[test]
    fn test_tool_name() {
        let tool = CalendarReschedule;
        assert_eq!(tool.name(), "Reschedule Event");
    }

    #[test]
    fn test_tool_has_schema() {
        let tool = CalendarReschedule;
        let schema = tool.schema();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
        assert!(schema["properties"]["new_start_in_minutes"].is_object());
        assert!(schema["properties"]["new_start_at"].is_object());
        assert!(schema["properties"]["new_end_at"].is_object());
        assert!(schema["properties"]["new_duration_minutes"].is_object());
        assert!(schema["properties"]["calendar"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&json!("query")));
    }

    #[test]
    fn test_execute_empty_query() {
        let tool = CalendarReschedule;
        let result = tool.execute(json!({}));
        assert_eq!(result.status, "error");
        assert!(result.message.contains("required"));
    }

    #[test]
    fn test_execute_no_new_time() {
        let tool = CalendarReschedule;
        let result = tool.execute(json!({"query": "test"}));
        assert_eq!(result.status, "error");
        assert!(result.message.contains("new_start"));
    }

    #[test]
    fn test_build_reschedule_script_minutes() {
        let script = build_reschedule_script_minutes("team meeting", 60, None, None);
        assert!(script.contains("team meeting"));
        assert!(script.contains("60 * 60"));
        assert!(script.contains("set start date of foundEvent"));
        assert!(script.contains("originalDuration"));
    }

    #[test]
    fn test_build_reschedule_script_minutes_with_duration() {
        let script = build_reschedule_script_minutes("team meeting", 60, Some(90), None);
        assert!(script.contains("team meeting"));
        assert!(script.contains("90 * 60"));
    }

    #[test]
    fn test_build_reschedule_script_date() {
        let script = build_reschedule_script_date("team meeting", "2024-01-15T13:00:00", None, None, None);
        assert!(script.contains("team meeting"));
        assert!(script.contains("2024-01-15T13:00:00"));
        assert!(script.contains("yearStr"));
        assert!(script.contains("monthStr"));
    }

    #[test]
    fn test_build_reschedule_script_with_calendar() {
        let script = build_reschedule_script_minutes("team meeting", 60, None, Some("Work"));
        assert!(script.contains("name of cal is \"Work\""));
    }
}
