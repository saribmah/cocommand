//! Calendar list_upcoming tool.
//!
//! This module provides the tool for listing upcoming calendar events.
//! Uses EventKit via Swift for reliable date-range queries across all calendar types.

use serde_json::{json, Value};

use crate::applications::types::{Tool, ToolResult};

use super::script::run_swift_script;

/// Tool ID for the list_upcoming tool.
pub const TOOL_ID: &str = "calendar_list_upcoming";

/// Tool for listing upcoming calendar events.
pub struct CalendarListUpcoming;

impl Tool for CalendarListUpcoming {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "List Upcoming"
    }

    fn description(&self) -> &str {
        "List upcoming calendar events from Apple Calendar. Returns events sorted by start date (earliest first). Use 'limit' to control how many events to return and 'days_ahead' to specify the time range."
    }

    fn schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "limit": {
                    "type": "integer",
                    "description": "Optional: Maximum number of events to return. Defaults to 10."
                },
                "days_ahead": {
                    "type": "integer",
                    "description": "Optional: Number of days ahead to look for events. Defaults to 7."
                },
                "calendar": {
                    "type": "string",
                    "description": "Optional: Name of a specific calendar to filter by (e.g., 'Work', 'Personal'). If not specified, returns events from all calendars."
                }
            }
        }))
    }

    fn execute(&self, inputs: Value) -> ToolResult {
        let limit = inputs
            .get("limit")
            .and_then(|v| v.as_i64())
            .unwrap_or(10) as usize;

        let days_ahead = inputs
            .get("days_ahead")
            .and_then(|v| v.as_i64())
            .unwrap_or(7);

        let calendar = inputs
            .get("calendar")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let script = build_swift_script(limit, days_ahead, calendar);
        let result = run_swift_script(&script);

        if result.status == "ok" {
            if result.message.trim().is_empty() {
                ToolResult::ok("No upcoming events found.")
            } else {
                result
            }
        } else {
            result
        }
    }
}

/// Build a Swift script that uses EventKit to list upcoming calendar events.
///
/// EventKit provides reliable date-range queries that work across all calendar types
/// (iCloud, Google, Exchange, etc.) unlike AppleScript which has filtering issues.
fn build_swift_script(limit: usize, days_ahead: i64, calendar: Option<&str>) -> String {
    let calendar_filter = match calendar {
        Some(c) => format!(
            r#"
let calendarFilter: String? = "{}"
"#,
            c.replace('\\', "\\\\").replace('"', "\\\"")
        ),
        None => r#"
let calendarFilter: String? = nil
"#
        .to_string(),
    };

    format!(
        r#"import EventKit
import Foundation

let store = EKEventStore()
let semaphore = DispatchSemaphore(value: 0)
var accessGranted = false

if #available(macOS 14.0, *) {{
    store.requestFullAccessToEvents {{ granted, error in
        accessGranted = granted
        semaphore.signal()
    }}
}} else {{
    store.requestAccess(to: .event) {{ granted, error in
        accessGranted = granted
        semaphore.signal()
    }}
}}

semaphore.wait()

if !accessGranted {{
    print("error:Calendar access not granted. Please grant access in System Settings > Privacy & Security > Calendars.")
    exit(1)
}}

let startDate = Date()
let daysAhead = {}
let endDate = Calendar.current.date(byAdding: .day, value: daysAhead, to: startDate)!
let limit = {}
{}
// Get calendars - optionally filter by name
var calendars: [EKCalendar]?
if let filterName = calendarFilter {{
    let allCalendars = store.calendars(for: .event)
    let matching = allCalendars.filter {{ $0.title == filterName }}
    if matching.isEmpty {{
        print("NOT_FOUND:Calendar '\(filterName)' not found. Available calendars: \(allCalendars.map {{ $0.title }}.joined(separator: ", "))")
        exit(1)
    }}
    calendars = matching
}}

let predicate = store.predicateForEvents(withStart: startDate, end: endDate, calendars: calendars)
let events = store.events(matching: predicate).sorted {{ $0.startDate < $1.startDate }}

if events.isEmpty {{
    print("")
    exit(0)
}}

let formatter = DateFormatter()
formatter.dateFormat = "MMM d, yyyy 'at' h:mm a"

let timeFormatter = DateFormatter()
timeFormatter.dateFormat = "h:mm a"

for event in events.prefix(limit) {{
    let title = event.title ?? "Untitled"
    let startStr = formatter.string(from: event.startDate)
    let endStr = timeFormatter.string(from: event.endDate)
    let calName = event.calendar.title
    let location = event.location ?? ""

    var line = "- \(title) (\(startStr) - \(endStr))"
    if !location.isEmpty {{
        line += " @ \(location)"
    }}
    line += " [\(calName)]"
    print(line)
}}
"#,
        days_ahead, limit, calendar_filter
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = CalendarListUpcoming;
        assert_eq!(tool.id(), "calendar_list_upcoming");
    }

    #[test]
    fn test_tool_name() {
        let tool = CalendarListUpcoming;
        assert_eq!(tool.name(), "List Upcoming");
    }

    #[test]
    fn test_tool_has_schema() {
        let tool = CalendarListUpcoming;
        let schema = tool.schema();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["limit"].is_object());
        assert!(schema["properties"]["days_ahead"].is_object());
        assert!(schema["properties"]["calendar"].is_object());
    }

    #[test]
    fn test_build_swift_script_defaults() {
        let script = build_swift_script(10, 7, None);
        assert!(script.contains("let daysAhead = 7"));
        assert!(script.contains("let limit = 10"));
        assert!(script.contains("let calendarFilter: String? = nil"));
        // Verify EventKit usage
        assert!(script.contains("import EventKit"));
        assert!(script.contains("predicateForEvents"));
    }

    #[test]
    fn test_build_swift_script_with_calendar() {
        let script = build_swift_script(5, 14, Some("Work"));
        assert!(script.contains("let daysAhead = 14"));
        assert!(script.contains("let limit = 5"));
        assert!(script.contains("let calendarFilter: String? = \"Work\""));
    }

    #[test]
    fn test_build_swift_script_escapes_quotes() {
        let script = build_swift_script(10, 7, Some("My \"Special\" Calendar"));
        assert!(script.contains("let calendarFilter: String? = \"My \\\"Special\\\" Calendar\""));
    }
}
