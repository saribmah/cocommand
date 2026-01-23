//! Calendar add tool.
//!
//! This module provides the tool for adding new calendar events.

use serde_json::{json, Value};

use crate::applications::types::{Tool, ToolResult};

use super::script::{escape_applescript_string, run_applescript_with_message};

/// Tool ID for the add tool.
pub const TOOL_ID: &str = "calendar_add";

/// Tool for adding a new calendar event.
pub struct CalendarAdd;

impl Tool for CalendarAdd {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Add Event"
    }

    fn description(&self) -> &str {
        "Add a new event to Apple Calendar. Use 'start_in_minutes' for relative time (e.g., 60 for '1 hour from now') or 'start_at' for an ISO 8601 date/time string (e.g., '2024-01-15T13:00:00'). Optionally specify 'end_at' or 'duration_minutes' for event duration. If neither is provided, defaults to 1 hour."
    }

    fn schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "The title/summary of the event (e.g., 'Team Meeting', 'Lunch with John', 'Doctor Appointment')"
                },
                "start_in_minutes": {
                    "type": "integer",
                    "description": "Optional: Number of minutes from now when the event starts (e.g., 60 for '1 hour from now', 1440 for 'tomorrow at same time')"
                },
                "start_at": {
                    "type": "string",
                    "description": "Optional: ISO 8601 date/time string for when the event starts (e.g., '2024-01-15T13:00:00'). The format must be YYYY-MM-DDTHH:MM:SS. Use start_in_minutes for relative times instead."
                },
                "end_at": {
                    "type": "string",
                    "description": "Optional: ISO 8601 date/time string for when the event ends (e.g., '2024-01-15T14:00:00'). The format must be YYYY-MM-DDTHH:MM:SS."
                },
                "duration_minutes": {
                    "type": "integer",
                    "description": "Optional: Duration of the event in minutes (e.g., 30 for a 30-minute meeting). Defaults to 60 if neither end_at nor duration_minutes is specified."
                },
                "calendar": {
                    "type": "string",
                    "description": "Optional: Name of the calendar to add the event to (e.g., 'Work', 'Personal'). Uses the first available calendar if not specified."
                },
                "location": {
                    "type": "string",
                    "description": "Optional: Location of the event (e.g., 'Conference Room A', '123 Main St')"
                },
                "notes": {
                    "type": "string",
                    "description": "Optional: Additional notes or description for the event"
                },
                "all_day": {
                    "type": "boolean",
                    "description": "Optional: Whether this is an all-day event. Defaults to false."
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
            return ToolResult::error("Event title is required");
        }

        let escaped_title = escape_applescript_string(title);

        let calendar = inputs
            .get("calendar")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let location = inputs
            .get("location")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let notes = inputs
            .get("notes")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let all_day = inputs
            .get("all_day")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let start_in_minutes = inputs.get("start_in_minutes").and_then(|v| v.as_i64());
        let start_at = inputs
            .get("start_at")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let end_at = inputs
            .get("end_at")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let duration_minutes = inputs
            .get("duration_minutes")
            .and_then(|v| v.as_i64())
            .unwrap_or(60); // Default to 1 hour

        // At least start_in_minutes or start_at must be provided
        if start_in_minutes.is_none() && start_at.is_none() {
            return ToolResult::error(
                "Either 'start_in_minutes' or 'start_at' is required to schedule the event",
            );
        }

        // Build the AppleScript
        let script = if let Some(minutes) = start_in_minutes {
            let escaped_calendar = calendar.map(|c| escape_applescript_string(c));
            let escaped_location = location.map(|l| escape_applescript_string(l));
            let escaped_notes = notes.map(|n| escape_applescript_string(n));
            build_add_script_with_minutes(
                &escaped_title,
                minutes,
                duration_minutes,
                escaped_calendar.as_deref(),
                escaped_location.as_deref(),
                escaped_notes.as_deref(),
                all_day,
            )
        } else {
            let escaped_start = escape_applescript_string(start_at.unwrap());
            let escaped_end = end_at.map(|e| escape_applescript_string(e));
            let escaped_calendar = calendar.map(|c| escape_applescript_string(c));
            let escaped_location = location.map(|l| escape_applescript_string(l));
            let escaped_notes = notes.map(|n| escape_applescript_string(n));
            build_add_script_with_date(
                &escaped_title,
                &escaped_start,
                escaped_end.as_deref(),
                duration_minutes,
                escaped_calendar.as_deref(),
                escaped_location.as_deref(),
                escaped_notes.as_deref(),
                all_day,
            )
        };

        let success_msg = if start_in_minutes.is_some() {
            format!(
                "Event '{}' added, starting in {} minute(s).",
                title,
                start_in_minutes.unwrap()
            )
        } else {
            format!("Event '{}' added, starting at {}.", title, start_at.unwrap())
        };

        run_applescript_with_message(&script, &success_msg)
    }
}

fn build_add_script_with_minutes(
    title: &str,
    start_minutes: i64,
    duration_minutes: i64,
    calendar: Option<&str>,
    location: Option<&str>,
    notes: Option<&str>,
    all_day: bool,
) -> String {
    let calendar_clause = match calendar {
        Some(c) => format!("calendar \"{}\"", c),
        None => "first calendar".to_string(),
    };

    let location_prop = match location {
        Some(l) => format!(", location:\"{}\"", l),
        None => String::new(),
    };

    let notes_prop = match notes {
        Some(n) => format!(", description:\"{}\"", n),
        None => String::new(),
    };

    let allday_prop = if all_day {
        ", allday event:true".to_string()
    } else {
        String::new()
    };

    format!(
        r#"
tell application "Calendar"
    set startDate to (current date) + ({} * 60)
    set endDate to startDate + ({} * 60)
    tell {}
        make new event with properties {{summary:"{}", start date:startDate, end date:endDate{}{}{}}}
    end tell
end tell
"#,
        start_minutes, duration_minutes, calendar_clause, title, location_prop, notes_prop, allday_prop
    )
}

fn build_add_script_with_date(
    title: &str,
    start_date_str: &str,
    end_date_str: Option<&str>,
    duration_minutes: i64,
    calendar: Option<&str>,
    location: Option<&str>,
    notes: Option<&str>,
    all_day: bool,
) -> String {
    let calendar_clause = match calendar {
        Some(c) => format!("calendar \"{}\"", c),
        None => "first calendar".to_string(),
    };

    let location_prop = match location {
        Some(l) => format!(", location:\"{}\"", l),
        None => String::new(),
    };

    let notes_prop = match notes {
        Some(n) => format!(", description:\"{}\"", n),
        None => String::new(),
    };

    let allday_prop = if all_day {
        ", allday event:true".to_string()
    } else {
        String::new()
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

    set endDate to current date
    set year of endDate to (endYearStr as integer)
    set month of endDate to (endMonthStr as integer)
    set day of endDate to (endDayStr as integer)
    set hours of endDate to (endHourStr as integer)
    set minutes of endDate to (endMinuteStr as integer)
    set seconds of endDate to (endSecondStr as integer)
"#,
            end_str
        ),
        None => format!(
            r#"
    set endDate to startDate + ({} * 60)
"#,
            duration_minutes
        ),
    };

    // Parse ISO 8601 format and construct AppleScript date explicitly
    // Expected format: YYYY-MM-DDTHH:MM:SS
    format!(
        r#"
tell application "Calendar"
    set isoString to "{}"

    -- Parse ISO 8601 format: YYYY-MM-DDTHH:MM:SS
    set yearStr to text 1 thru 4 of isoString
    set monthStr to text 6 thru 7 of isoString
    set dayStr to text 9 thru 10 of isoString
    set hourStr to text 12 thru 13 of isoString
    set minuteStr to text 15 thru 16 of isoString
    set secondStr to text 18 thru 19 of isoString

    -- Create start date object
    set startDate to current date
    set year of startDate to (yearStr as integer)
    set month of startDate to (monthStr as integer)
    set day of startDate to (dayStr as integer)
    set hours of startDate to (hourStr as integer)
    set minutes of startDate to (minuteStr as integer)
    set seconds of startDate to (secondStr as integer)
{}
    tell {}
        make new event with properties {{summary:"{}", start date:startDate, end date:endDate{}{}{}}}
    end tell
end tell
"#,
        start_date_str, end_date_script, calendar_clause, title, location_prop, notes_prop, allday_prop
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = CalendarAdd;
        assert_eq!(tool.id(), "calendar_add");
    }

    #[test]
    fn test_tool_name() {
        let tool = CalendarAdd;
        assert_eq!(tool.name(), "Add Event");
    }

    #[test]
    fn test_tool_has_schema() {
        let tool = CalendarAdd;
        let schema = tool.schema();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["title"].is_object());
        assert!(schema["properties"]["start_in_minutes"].is_object());
        assert!(schema["properties"]["start_at"].is_object());
        assert!(schema["properties"]["end_at"].is_object());
        assert!(schema["properties"]["duration_minutes"].is_object());
        assert!(schema["properties"]["calendar"].is_object());
        assert!(schema["properties"]["location"].is_object());
        assert!(schema["properties"]["notes"].is_object());
        assert!(schema["properties"]["all_day"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&json!("title")));
    }

    #[test]
    fn test_execute_empty_title() {
        let tool = CalendarAdd;
        let result = tool.execute(json!({}));
        assert_eq!(result.status, "error");
        assert!(result.message.contains("required"));
    }

    #[test]
    fn test_execute_no_start_time() {
        let tool = CalendarAdd;
        let result = tool.execute(json!({"title": "Test Event"}));
        assert_eq!(result.status, "error");
        assert!(result.message.contains("start_in_minutes") || result.message.contains("start_at"));
    }

    #[test]
    fn test_build_script_with_minutes() {
        let script = build_add_script_with_minutes("Test event", 60, 30, None, None, None, false);
        assert!(script.contains("Test event"));
        assert!(script.contains("60 * 60"));
        assert!(script.contains("30 * 60"));
        assert!(script.contains("first calendar"));
    }

    #[test]
    fn test_build_script_with_date() {
        let script = build_add_script_with_date(
            "Test event",
            "2024-01-15T13:00:00",
            None,
            60,
            None,
            None,
            None,
            false,
        );
        assert!(script.contains("Test event"));
        assert!(script.contains("2024-01-15T13:00:00"));
        assert!(script.contains("first calendar"));
        assert!(script.contains("yearStr"));
        assert!(script.contains("monthStr"));
    }

    #[test]
    fn test_build_script_with_custom_calendar() {
        let script = build_add_script_with_minutes("Test event", 60, 30, Some("Work"), None, None, false);
        assert!(script.contains("Test event"));
        assert!(script.contains("calendar \"Work\""));
    }

    #[test]
    fn test_build_script_with_location() {
        let script = build_add_script_with_minutes(
            "Test event",
            60,
            30,
            None,
            Some("Conference Room"),
            None,
            false,
        );
        assert!(script.contains("location:\"Conference Room\""));
    }

    #[test]
    fn test_build_script_with_notes() {
        let script = build_add_script_with_minutes(
            "Test event",
            60,
            30,
            None,
            None,
            Some("Remember to bring documents"),
            false,
        );
        assert!(script.contains("description:\"Remember to bring documents\""));
    }

    #[test]
    fn test_build_script_all_day() {
        let script = build_add_script_with_minutes("Test event", 60, 30, None, None, None, true);
        assert!(script.contains("allday event:true"));
    }
}
