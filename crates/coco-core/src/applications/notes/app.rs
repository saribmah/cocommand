//! Notes application definition.
//!
//! This module provides the NotesApp struct that implements the Application trait.

use crate::applications::types::{tool_definition, Application, ToolDefinition};

use super::add::NotesAdd;
use super::delete::NotesDelete;
use super::list::NotesList;
use super::open::NotesOpen;
use super::summarize::NotesSummarize;

/// Application ID for Notes.
pub const APP_ID: &str = "notes";

/// Apple Notes application.
#[derive(Default)]
pub struct NotesApp;

impl Application for NotesApp {
    fn id(&self) -> &str {
        APP_ID
    }

    fn name(&self) -> &str {
        "Notes"
    }

    fn description(&self) -> &str {
        "Manage notes using Apple Notes. Add, list, delete, and summarize notes with automatic categorization."
    }

    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            tool_definition(&NotesOpen),
            tool_definition(&NotesAdd),
            tool_definition(&NotesList),
            tool_definition(&NotesDelete),
            tool_definition(&NotesSummarize),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_id() {
        let app = NotesApp::default();
        assert_eq!(app.id(), "notes");
    }

    #[test]
    fn test_app_name() {
        let app = NotesApp::default();
        assert_eq!(app.name(), "Notes");
    }

    #[test]
    fn test_app_tools() {
        let app = NotesApp::default();
        let tools = app.tools();

        assert_eq!(tools.len(), 5);
        assert!(tools.iter().any(|t| t.id == "notes_open"));
        assert!(tools.iter().any(|t| t.id == "notes_add"));
        assert!(tools.iter().any(|t| t.id == "notes_list"));
        assert!(tools.iter().any(|t| t.id == "notes_delete"));
        assert!(tools.iter().any(|t| t.id == "notes_summarize"));
    }

    #[test]
    fn test_add_has_schema() {
        let app = NotesApp::default();
        let tools = app.tools();
        let add_tool = tools.iter().find(|t| t.id == "notes_add").unwrap();

        assert!(add_tool.schema.is_some());
        let schema = add_tool.schema.as_ref().unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["content"].is_object());
    }

    #[test]
    fn test_list_has_schema() {
        let app = NotesApp::default();
        let tools = app.tools();
        let list_tool = tools.iter().find(|t| t.id == "notes_list").unwrap();

        assert!(list_tool.schema.is_some());
        let schema = list_tool.schema.as_ref().unwrap();
        assert_eq!(schema["type"], "object");
    }

    #[test]
    fn test_delete_has_schema() {
        let app = NotesApp::default();
        let tools = app.tools();
        let delete_tool = tools.iter().find(|t| t.id == "notes_delete").unwrap();

        assert!(delete_tool.schema.is_some());
        let schema = delete_tool.schema.as_ref().unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
    }
}
