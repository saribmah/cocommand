//! Notes open tool.
//!
//! This module provides the tool for opening/activating the Notes application.

use serde_json::Value;

use crate::applications::types::{Tool, ToolResult};

use super::script::run_notes_script;

/// Tool ID for the open tool.
pub const TOOL_ID: &str = "notes_open";

/// Tool for opening/activating the Notes application.
pub struct NotesOpen;

impl Tool for NotesOpen {
    fn id(&self) -> &str {
        TOOL_ID
    }

    fn name(&self) -> &str {
        "Open"
    }

    fn description(&self) -> &str {
        "Open and activate the Apple Notes application. Launches Notes if not running, or brings it to focus if already open."
    }

    fn execute(&self, _inputs: Value) -> ToolResult {
        run_notes_script("activate", "Notes app activated.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_id() {
        let tool = NotesOpen;
        assert_eq!(tool.id(), "notes_open");
    }

    #[test]
    fn test_tool_name() {
        let tool = NotesOpen;
        assert_eq!(tool.name(), "Open");
    }

    #[test]
    fn test_tool_description() {
        let tool = NotesOpen;
        assert!(tool.description().contains("Open"));
        assert!(tool.description().contains("Notes"));
    }
}
