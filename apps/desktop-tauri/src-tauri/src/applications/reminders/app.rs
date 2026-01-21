//! Reminders application definition.
//!
//! This module provides the RemindersApp struct that implements the Application trait.

use crate::applications::types::{tool_definition, Application, ToolDefinition};

use super::add::RemindersAdd;
use super::cancel::RemindersCancel;
use super::complete::RemindersComplete;
use super::list_upcoming::RemindersListUpcoming;
use super::open::RemindersOpen;
use super::reschedule::RemindersReschedule;

/// Application ID for Reminders.
pub const APP_ID: &str = "reminders";

/// Apple Reminders application.
#[derive(Default)]
pub struct RemindersApp;

impl Application for RemindersApp {
    fn id(&self) -> &str {
        APP_ID
    }

    fn name(&self) -> &str {
        "Reminders"
    }

    fn description(&self) -> &str {
        "Manage reminders using Apple Reminders. Add, list, complete, cancel, and reschedule reminders."
    }

    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            tool_definition(&RemindersOpen),
            tool_definition(&RemindersAdd),
            tool_definition(&RemindersListUpcoming),
            tool_definition(&RemindersCancel),
            tool_definition(&RemindersReschedule),
            tool_definition(&RemindersComplete),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_id() {
        let app = RemindersApp::default();
        assert_eq!(app.id(), "reminders");
    }

    #[test]
    fn test_app_name() {
        let app = RemindersApp::default();
        assert_eq!(app.name(), "Reminders");
    }

    #[test]
    fn test_app_tools() {
        let app = RemindersApp::default();
        let tools = app.tools();

        assert_eq!(tools.len(), 6);
        assert!(tools.iter().any(|t| t.id == "reminders_open"));
        assert!(tools.iter().any(|t| t.id == "reminders_add"));
        assert!(tools.iter().any(|t| t.id == "reminders_list_upcoming"));
        assert!(tools.iter().any(|t| t.id == "reminders_cancel"));
        assert!(tools.iter().any(|t| t.id == "reminders_reschedule"));
        assert!(tools.iter().any(|t| t.id == "reminders_complete"));
    }

    #[test]
    fn test_add_has_schema() {
        let app = RemindersApp::default();
        let tools = app.tools();
        let add_tool = tools.iter().find(|t| t.id == "reminders_add").unwrap();

        assert!(add_tool.schema.is_some());
        let schema = add_tool.schema.as_ref().unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["title"].is_object());
    }

    #[test]
    fn test_list_upcoming_has_schema() {
        let app = RemindersApp::default();
        let tools = app.tools();
        let list_tool = tools
            .iter()
            .find(|t| t.id == "reminders_list_upcoming")
            .unwrap();

        assert!(list_tool.schema.is_some());
        let schema = list_tool.schema.as_ref().unwrap();
        assert_eq!(schema["type"], "object");
    }

    #[test]
    fn test_cancel_has_schema() {
        let app = RemindersApp::default();
        let tools = app.tools();
        let cancel_tool = tools.iter().find(|t| t.id == "reminders_cancel").unwrap();

        assert!(cancel_tool.schema.is_some());
        let schema = cancel_tool.schema.as_ref().unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
    }

    #[test]
    fn test_reschedule_has_schema() {
        let app = RemindersApp::default();
        let tools = app.tools();
        let reschedule_tool = tools
            .iter()
            .find(|t| t.id == "reminders_reschedule")
            .unwrap();

        assert!(reschedule_tool.schema.is_some());
        let schema = reschedule_tool.schema.as_ref().unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
    }

    #[test]
    fn test_complete_has_schema() {
        let app = RemindersApp::default();
        let tools = app.tools();
        let complete_tool = tools.iter().find(|t| t.id == "reminders_complete").unwrap();

        assert!(complete_tool.schema.is_some());
        let schema = complete_tool.schema.as_ref().unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
    }
}
