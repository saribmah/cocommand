use crate::application::{Application, ApplicationAction, ApplicationKind};

#[derive(Debug, Default)]
pub struct NoteApplication;

impl NoteApplication {
    pub fn new() -> Self {
        Self
    }
}

impl Application for NoteApplication {
    fn id(&self) -> &str {
        "notes"
    }

    fn name(&self) -> &str {
        "Notes"
    }

    fn kind(&self) -> ApplicationKind {
        ApplicationKind::BuiltIn
    }

    fn tags(&self) -> Vec<String> {
        vec!["notes".to_string(), "writing".to_string()]
    }

    fn actions(&self) -> Vec<ApplicationAction> {
        vec![
            ApplicationAction {
                id: "create-note".to_string(),
                name: "Create Note".to_string(),
                description: Some("Create a new note".to_string()),
            },
            ApplicationAction {
                id: "list-notes".to_string(),
                name: "List Notes".to_string(),
                description: Some("Show recent notes".to_string()),
            },
        ]
    }
}
