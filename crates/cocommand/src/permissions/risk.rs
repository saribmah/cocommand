//! Mapping from tool risk levels to permission scopes.

use crate::tools::RiskLevel;
use super::scopes::PermissionScope;

/// Derive the required permission scope from a tool's risk level.
pub fn risk_for_tool(risk_level: &RiskLevel) -> PermissionScope {
    match risk_level {
        RiskLevel::Safe => PermissionScope::Read,
        RiskLevel::Confirm => PermissionScope::Write,
        RiskLevel::Destructive => PermissionScope::Execute,
    }
}

/// Whether this risk level requires user confirmation before execution.
pub fn requires_confirmation(risk_level: &RiskLevel) -> bool {
    match risk_level {
        RiskLevel::Safe => false,
        RiskLevel::Confirm | RiskLevel::Destructive => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_maps_to_read() {
        assert_eq!(risk_for_tool(&RiskLevel::Safe), PermissionScope::Read);
    }

    #[test]
    fn confirm_maps_to_write() {
        assert_eq!(risk_for_tool(&RiskLevel::Confirm), PermissionScope::Write);
    }

    #[test]
    fn destructive_maps_to_execute() {
        assert_eq!(risk_for_tool(&RiskLevel::Destructive), PermissionScope::Execute);
    }

    #[test]
    fn safe_does_not_require_confirmation() {
        assert!(!requires_confirmation(&RiskLevel::Safe));
    }

    #[test]
    fn confirm_requires_confirmation() {
        assert!(requires_confirmation(&RiskLevel::Confirm));
    }

    #[test]
    fn destructive_requires_confirmation() {
        assert!(requires_confirmation(&RiskLevel::Destructive));
    }
}
