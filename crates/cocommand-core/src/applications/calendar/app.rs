//! Calendar application definition.
//!
//! This module provides the CalendarApp struct that implements the Application trait.

use crate::applications::types::{tool_definition, Application, ToolDefinition};

use super::add::CalendarAdd;
use super::cancel::CalendarCancel;
use super::list_upcoming::CalendarListUpcoming;
use super::open::CalendarOpen;
use super::reschedule::CalendarReschedule;

/// Application ID for Calendar.
pub const APP_ID: &str = "calendar";

/// Apple Calendar application.
#[derive(Default)]
pub struct CalendarApp;

impl Application for CalendarApp {
    fn id(&self) -> &str {
        APP_ID
    }

    fn name(&self) -> &str {
        "Calendar"
    }

    fn description(&self) -> &str {
        "Manage calendar events using Apple Calendar. Add, list, cancel, and reschedule events."
    }

    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            tool_definition(&CalendarOpen),
            tool_definition(&CalendarAdd),
            tool_definition(&CalendarListUpcoming),
            tool_definition(&CalendarCancel),
            tool_definition(&CalendarReschedule),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_id() {
        let app = CalendarApp::default();
        assert_eq!(app.id(), "calendar");
    }

    #[test]
    fn test_app_name() {
        let app = CalendarApp::default();
        assert_eq!(app.name(), "Calendar");
    }

    #[test]
    fn test_app_tools() {
        let app = CalendarApp::default();
        let tools = app.tools();

        assert_eq!(tools.len(), 5);
        assert!(tools.iter().any(|t| t.id == "calendar_open"));
        assert!(tools.iter().any(|t| t.id == "calendar_add"));
        assert!(tools.iter().any(|t| t.id == "calendar_list_upcoming"));
        assert!(tools.iter().any(|t| t.id == "calendar_cancel"));
        assert!(tools.iter().any(|t| t.id == "calendar_reschedule"));
    }

    #[test]
    fn test_add_has_schema() {
        let app = CalendarApp::default();
        let tools = app.tools();
        let add_tool = tools.iter().find(|t| t.id == "calendar_add").unwrap();

        assert!(add_tool.schema.is_some());
        let schema = add_tool.schema.as_ref().unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["title"].is_object());
    }

    #[test]
    fn test_list_upcoming_has_schema() {
        let app = CalendarApp::default();
        let tools = app.tools();
        let list_tool = tools
            .iter()
            .find(|t| t.id == "calendar_list_upcoming")
            .unwrap();

        assert!(list_tool.schema.is_some());
        let schema = list_tool.schema.as_ref().unwrap();
        assert_eq!(schema["type"], "object");
    }

    #[test]
    fn test_cancel_has_schema() {
        let app = CalendarApp::default();
        let tools = app.tools();
        let cancel_tool = tools.iter().find(|t| t.id == "calendar_cancel").unwrap();

        assert!(cancel_tool.schema.is_some());
        let schema = cancel_tool.schema.as_ref().unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
    }

    #[test]
    fn test_reschedule_has_schema() {
        let app = CalendarApp::default();
        let tools = app.tools();
        let reschedule_tool = tools
            .iter()
            .find(|t| t.id == "calendar_reschedule")
            .unwrap();

        assert!(reschedule_tool.schema.is_some());
        let schema = reschedule_tool.schema.as_ref().unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
    }
}
