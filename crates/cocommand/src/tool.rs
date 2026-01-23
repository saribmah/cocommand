//! Tool module for the cocommand command bar.
//!
//! This module manages tool registration and provides builders for:
//! - Control plane tool sets (window.* tools only)
//! - Execution plane tool sets (window.* + app tools)
//!
//! # Architecture
//!
//! Tools are organized into domains:
//! - `window`: Window management tools (always available)
//! - App-specific tools are mounted dynamically when apps are opened
//!
//! The registry provides builders for different execution phases:
//! - Control phase: Only window.* tools available
//! - Execution phase: Window.* plus open app tools

pub mod registry;
pub mod window;

// Re-export commonly used items
pub use registry::{build_archived_tool_set, build_control_plane_tool_set, build_execution_plane_tool_set, build_tool_set};
pub use window::{all_tool_ids as all_window_tool_ids, build_archived_tools, build_window_tools, is_window_tool};
