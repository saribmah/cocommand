//! Calendar open tool.
//!
//! This module provides the tool for opening/activating the Calendar application.

use serde_json::Value;

use crate::applications::types::{Tool, ToolResult};

use super::script::run_calendar_script;

/// Tool ID for the open tool.
pub const TOOL_ID: &str = "calendar_open";

/// Tool for opening/activating the Calendar application.
pub struct CalendarOpen;

impl Tool for CalendarOpen {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Open"
    }

    fn description(&self) -> &str {
        "Open and activate the Apple Calendar application. Launches Calendar if not running, or brings it to focus if already open."
    }

    fn execute(&self, _inputs: Value) -> ToolResult {
        run_calendar_script("activate", "Calendar app activated.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = CalendarOpen;
        assert_eq!(tool.id(), "calendar_open");
    }

    #[test]
    fn test_tool_name() {
        let tool = CalendarOpen;
        assert_eq!(tool.name(), "Open");
    }

    #[test]
    fn test_tool_description() {
        let tool = CalendarOpen;
        assert!(tool.description().contains("Open"));
        assert!(tool.description().contains("Calendar"));
    }
}
