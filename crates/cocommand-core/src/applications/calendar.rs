//! Apple Calendar application integration.
//!
//! Provides tools for managing calendar events via AppleScript on macOS.
//!
//! # Submodules
//!
//! - `app`: CalendarApp implementation
//! - `open`: Open/activate Calendar app
//! - `add`: Add new calendar event with optional end time and location
//! - `list_upcoming`: List upcoming calendar events
//! - `cancel`: Delete/cancel a calendar event by name
//! - `reschedule`: Update a calendar event's start/end time
//! - `script`: Shared AppleScript execution utilities
//!
//! # Usage
//!
//! The CalendarApp is registered in the applications registry and provides
//! tools: `calendar_open`, `calendar_add`, `calendar_list_upcoming`,
//! `calendar_cancel`, and `calendar_reschedule`.
//! These are mounted when the Calendar app is opened via `window.open`.

pub mod add;
pub mod app;
pub mod cancel;
pub mod list_upcoming;
pub mod open;
pub mod reschedule;
pub mod script;

// Re-export commonly used items
pub use add::{CalendarAdd, TOOL_ID as ADD_TOOL_ID};
pub use app::{CalendarApp, APP_ID};
pub use cancel::{CalendarCancel, TOOL_ID as CANCEL_TOOL_ID};
pub use list_upcoming::{CalendarListUpcoming, TOOL_ID as LIST_UPCOMING_TOOL_ID};
pub use open::{CalendarOpen, TOOL_ID as OPEN_TOOL_ID};
pub use reschedule::{CalendarReschedule, TOOL_ID as RESCHEDULE_TOOL_ID};
