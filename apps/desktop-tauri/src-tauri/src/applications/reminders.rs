//! Apple Reminders application integration.
//!
//! Provides tools for managing reminders via AppleScript on macOS.
//!
//! # Submodules
//!
//! - `app`: RemindersApp implementation
//! - `open`: Open/activate Reminders app
//! - `add`: Add new reminder with optional due date
//! - `list_upcoming`: List upcoming/incomplete reminders
//! - `cancel`: Delete/cancel a reminder by name
//! - `reschedule`: Update a reminder's due date
//! - `complete`: Mark a reminder as completed
//! - `script`: Shared AppleScript execution utilities
//!
//! # Usage
//!
//! The RemindersApp is registered in the applications registry and provides
//! tools: `reminders_open`, `reminders_add`, `reminders_list_upcoming`,
//! These are mounted when the Reminders app is opened via `window.open`.

pub mod add;
pub mod app;
pub mod cancel;
pub mod complete;
pub mod list_upcoming;
pub mod open;
pub mod reschedule;
pub mod script;

// Re-export commonly used items
pub use add::{RemindersAdd, TOOL_ID as ADD_TOOL_ID};
pub use app::{RemindersApp, APP_ID};
pub use cancel::{RemindersCancel, TOOL_ID as CANCEL_TOOL_ID};
pub use complete::{RemindersComplete, TOOL_ID as COMPLETE_TOOL_ID};
pub use list_upcoming::{RemindersListUpcoming, TOOL_ID as LIST_UPCOMING_TOOL_ID};
pub use open::{RemindersOpen, TOOL_ID as OPEN_TOOL_ID};
pub use reschedule::{RemindersReschedule, TOOL_ID as RESCHEDULE_TOOL_ID};
