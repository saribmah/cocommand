//! Tool registry module for the cocommand command bar.
//!
//! This module manages tool registration and provides builders for:
//! - Control plane tool sets (window.* tools only)
//! - Execution plane tool sets (window.* + app tools)
//!
//! The window tools are organized into submodules for modularity.

pub mod registry;
pub mod window;
