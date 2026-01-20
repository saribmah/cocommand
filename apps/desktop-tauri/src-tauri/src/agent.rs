//! Agent module for the cocommand command bar.
//!
//! This module provides the agent loop implementation with:
//! - Two-phase execution (control → execution)
//! - Prompt construction with modular components
//! - Context building with workspace lifecycle awareness
//! - Session management

pub mod config;
pub mod context;
pub mod processor;
pub mod prompt;
pub mod registry;
pub mod runner;
pub mod session;
