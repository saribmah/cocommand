//! Applications module for the cocommand command bar.
//!
//! This module provides the application abstraction layer:
//! - `types`: Core traits and data structures (Application, Tool, etc.)
//! - `registry`: Application and tool registration/execution
//! - `spotify`: Spotify application integration
//! - `reminders`: Apple Reminders application integration
//!
//! # Architecture
//!
//! Applications provide tool bundles that are dynamically mounted when
//! the application is opened via `window.open`. Each application implements
//! the `Application` trait and provides a list of `Tool` implementations.

pub mod registry;
pub mod reminders;
pub mod spotify;
pub mod types;

// Re-export commonly used items for convenience
pub use registry::{all_apps, all_tools, app_by_id, app_id_from_tool, execute_tool, tools_for_app};
pub use types::{
    tool_definition, Application, ApplicationDefinition, Tool, ToolDefinition, ToolResult,
};
