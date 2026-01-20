//! Agent module for the cocommand command bar.
//!
//! This module provides the agent loop implementation with:
//! - Two-phase execution (control → execution)
//! - Prompt construction with modular components
//! - Context building with workspace lifecycle awareness
//! - Session management
//! - System prompt assembly
//!
//! # Submodules
//!
//! - `config`: Agent configuration and settings
//! - `context`: Workspace lifecycle-aware context building
//! - `processor`: Main control→execution loop
//! - `prompt`: Modular prompt construction
//! - `registry`: Agent defaults and factories
//! - `runner`: Legacy agent runner (compatibility)
//! - `session`: Session state and message types (split into submodules)
//! - `system`: Pure system prompt assembly (opencode-style)

pub mod config;
pub mod context;
pub mod processor;
pub mod prompt;
pub mod registry;
pub mod runner;
pub mod session;
pub mod system;
