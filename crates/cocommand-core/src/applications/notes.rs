//! Apple Notes application integration.
//!
//! Provides tools for managing notes via AppleScript on macOS.
//!
//! # Submodules
//!
//! - `app`: NotesApp implementation
//! - `open`: Open/activate Notes app
//! - `add`: Add new note with automatic categorization
//! - `list`: List all notes with categories
//! - `delete`: Delete a note by name/content match
//! - `summarize`: Summarize all notes using LLM
//! - `script`: Shared AppleScript execution utilities
//!
//! # Usage
//!
//! The NotesApp is registered in the applications registry and provides
//! tools: `notes_open`, `notes_add`, `notes_list`, `notes_delete`, and
//! `notes_summarize`. These are mounted when the Notes app is opened via
//! `window.open`.

pub mod add;
pub mod app;
pub mod delete;
pub mod list;
pub mod open;
pub mod script;
pub mod summarize;

// Re-export commonly used items
pub use add::{NotesAdd, TOOL_ID as ADD_TOOL_ID};
pub use app::{NotesApp, APP_ID};
pub use delete::{NotesDelete, TOOL_ID as DELETE_TOOL_ID};
pub use list::{NotesList, TOOL_ID as LIST_TOOL_ID};
pub use open::{NotesOpen, TOOL_ID as OPEN_TOOL_ID};
pub use summarize::{NotesSummarize, TOOL_ID as SUMMARIZE_TOOL_ID};
