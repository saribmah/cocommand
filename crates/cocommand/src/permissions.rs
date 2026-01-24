//! Permission enforcement and confirmation flow (Core-4).

pub mod scopes;
pub mod risk;
pub mod store;
pub mod enforcement;

pub use scopes::PermissionScope;
pub use store::{PermissionDecision, PermissionStore};
pub use enforcement::{EnforcementResult, enforce_permissions};
