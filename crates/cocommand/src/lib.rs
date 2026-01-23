pub mod server;

pub mod error;
pub mod types;
pub mod core;
pub mod platform;

pub mod command;
pub mod routing;
pub mod planner;
pub mod workspace;
pub mod permissions;
pub mod tools;
pub mod events;
pub mod extensions;
pub mod builtins;

pub use crate::core::Core;
pub use crate::error::{CoreError, CoreResult};
pub use crate::types::{ActionSummary, ConfirmationDecision, CoreResponse};
pub use crate::workspace::Workspace;
