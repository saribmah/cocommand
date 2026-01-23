//! Permission scope definitions.

use serde::{Deserialize, Serialize};

/// The scope of permission required for a tool invocation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PermissionScope {
    /// Non-destructive data access.
    Read,
    /// Data modification.
    Write,
    /// Command/process execution.
    Execute,
    /// Workspace structure changes.
    WorkspaceManage,
}
