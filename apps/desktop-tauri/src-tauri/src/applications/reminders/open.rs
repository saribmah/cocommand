//! Reminders open tool.
//!
//! This module provides the tool for opening/activating the Reminders application.

use serde_json::Value;

use crate::applications::types::{Tool, ToolResult};

use super::script::run_reminders_script;

/// Tool ID for the open tool.
pub const TOOL_ID: &str = "reminders_open";

/// Tool for opening/activating the Reminders application.
pub struct RemindersOpen;

impl Tool for RemindersOpen {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Open"
    }

    fn description(&self) -> &str {
        "Open and activate the Apple Reminders application. Launches Reminders if not running, or brings it to focus if already open."
    }

    fn execute(&self, _inputs: Value) -> ToolResult {
        run_reminders_script("activate", "Reminders app activated.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = RemindersOpen;
        assert_eq!(tool.id(), "reminders_open");
    }

    #[test]
    fn test_tool_name() {
        let tool = RemindersOpen;
        assert_eq!(tool.name(), "Open");
    }

    #[test]
    fn test_tool_description() {
        let tool = RemindersOpen;
        assert!(tool.description().contains("Open"));
        assert!(tool.description().contains("Reminders"));
    }
}
