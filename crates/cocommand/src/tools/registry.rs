//! Tool registry for kernel and instance-scoped tools.

use std::collections::HashMap;

use crate::workspace::InstanceId;
use super::schema::ToolDefinition;

/// Registry holding kernel tools (global) and instance-scoped tools.
///
/// Lookup resolution: kernel tools take priority over instance tools with the same ID.
pub struct ToolRegistry {
    kernel_tools: HashMap<String, ToolDefinition>,
    instance_tools: HashMap<(InstanceId, String), ToolDefinition>,
}

impl ToolRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            kernel_tools: HashMap::new(),
            instance_tools: HashMap::new(),
        }
    }

    /// Register a kernel-level tool. Panics if a kernel tool with the same ID already exists.
    pub fn register_kernel_tool(&mut self, tool: ToolDefinition) {
        if self.kernel_tools.contains_key(&tool.tool_id) {
            panic!("duplicate kernel tool: {}", tool.tool_id);
        }
        self.kernel_tools.insert(tool.tool_id.clone(), tool);
    }

    /// Register an instance-scoped tool.
    pub fn register_instance_tool(&mut self, instance_id: InstanceId, tool: ToolDefinition) {
        let key = (instance_id, tool.tool_id.clone());
        self.instance_tools.insert(key, tool);
    }

    /// Remove all instance tools for a given instance.
    pub fn remove_instance_tools(&mut self, instance_id: &str) {
        self.instance_tools
            .retain(|(iid, _), _| iid != instance_id);
    }

    /// Look up a tool by instance context: kernel tools first, then instance tools.
    pub fn lookup(&self, instance_id: &str, tool_id: &str) -> Option<&ToolDefinition> {
        if let Some(tool) = self.kernel_tools.get(tool_id) {
            return Some(tool);
        }
        let key = (instance_id.to_string(), tool_id.to_string());
        self.instance_tools.get(&key)
    }

    /// Returns sorted list of available tool IDs for an instance (kernel + instance tools).
    pub fn available_tools(&self, instance_id: &str) -> Vec<&str> {
        let mut ids: Vec<&str> = self
            .kernel_tools
            .keys()
            .map(|s| s.as_str())
            .collect();
        ids.sort();

        let mut instance_ids: Vec<&str> = self
            .instance_tools
            .iter()
            .filter(|((iid, _), _)| iid == instance_id)
            .map(|((_, tid), _)| tid.as_str())
            .filter(|tid| !self.kernel_tools.contains_key(*tid))
            .collect();
        instance_ids.sort();

        ids.extend(instance_ids);
        ids
    }

    /// Returns a snapshot of kernel tools as (id, definition) pairs.
    pub fn kernel_tools(&self) -> Vec<(&str, &ToolDefinition)> {
        self.kernel_tools
            .iter()
            .map(|(id, def)| (id.as_str(), def))
            .collect()
    }

    /// Number of registered kernel tools.
    pub fn kernel_tool_count(&self) -> usize {
        self.kernel_tools.len()
    }

    /// Number of registered instance tools.
    pub fn instance_tool_count(&self) -> usize {
        self.instance_tools.len()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::schema::{RiskLevel, ToolHandler};
    use serde_json::json;

    fn make_handler() -> ToolHandler {
        Box::new(|_args, _ctx| Ok(json!({"ok": true})))
    }

    fn make_tool(id: &str, is_kernel: bool) -> ToolDefinition {
        ToolDefinition {
            tool_id: id.to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            risk_level: RiskLevel::Safe,
            is_kernel,
            handler: make_handler(),
        }
    }

    #[test]
    fn empty_registry() {
        let reg = ToolRegistry::new();
        assert_eq!(reg.kernel_tool_count(), 0);
        assert_eq!(reg.instance_tool_count(), 0);
        assert!(reg.lookup("any", "any").is_none());
        assert!(reg.available_tools("any").is_empty());
    }

    #[test]
    fn register_and_lookup_kernel_tool() {
        let mut reg = ToolRegistry::new();
        reg.register_kernel_tool(make_tool("read_file", true));

        assert_eq!(reg.kernel_tool_count(), 1);
        let tool = reg.lookup("any-instance", "read_file").unwrap();
        assert_eq!(tool.tool_id, "read_file");
        assert!(tool.is_kernel);
    }

    #[test]
    fn register_and_lookup_instance_tool() {
        let mut reg = ToolRegistry::new();
        reg.register_instance_tool("inst-1".to_string(), make_tool("custom_tool", false));

        assert_eq!(reg.instance_tool_count(), 1);
        let tool = reg.lookup("inst-1", "custom_tool").unwrap();
        assert_eq!(tool.tool_id, "custom_tool");
    }

    #[test]
    fn instance_tool_not_visible_to_other_instances() {
        let mut reg = ToolRegistry::new();
        reg.register_instance_tool("inst-1".to_string(), make_tool("private_tool", false));

        assert!(reg.lookup("inst-2", "private_tool").is_none());
    }

    #[test]
    fn kernel_takes_priority_over_instance_tool() {
        let mut reg = ToolRegistry::new();
        reg.register_kernel_tool(make_tool("shared_id", true));
        reg.register_instance_tool("inst-1".to_string(), make_tool("shared_id", false));

        let tool = reg.lookup("inst-1", "shared_id").unwrap();
        assert!(tool.is_kernel);
    }

    #[test]
    fn available_tools_returns_sorted_combined_list() {
        let mut reg = ToolRegistry::new();
        reg.register_kernel_tool(make_tool("b_kernel", true));
        reg.register_kernel_tool(make_tool("a_kernel", true));
        reg.register_instance_tool("inst-1".to_string(), make_tool("c_instance", false));
        reg.register_instance_tool("inst-1".to_string(), make_tool("a_instance", false));

        let tools = reg.available_tools("inst-1");
        assert_eq!(tools, vec!["a_kernel", "b_kernel", "a_instance", "c_instance"]);
    }

    #[test]
    fn remove_instance_tools_clears_correctly() {
        let mut reg = ToolRegistry::new();
        reg.register_instance_tool("inst-1".to_string(), make_tool("tool_a", false));
        reg.register_instance_tool("inst-1".to_string(), make_tool("tool_b", false));

        assert_eq!(reg.instance_tool_count(), 2);
        reg.remove_instance_tools("inst-1");
        assert_eq!(reg.instance_tool_count(), 0);
        assert!(reg.lookup("inst-1", "tool_a").is_none());
    }

    #[test]
    fn remove_does_not_affect_other_instances() {
        let mut reg = ToolRegistry::new();
        reg.register_instance_tool("inst-1".to_string(), make_tool("tool_a", false));
        reg.register_instance_tool("inst-2".to_string(), make_tool("tool_b", false));

        reg.remove_instance_tools("inst-1");
        assert_eq!(reg.instance_tool_count(), 1);
        assert!(reg.lookup("inst-2", "tool_b").is_some());
    }

    #[test]
    #[should_panic(expected = "duplicate kernel tool")]
    fn duplicate_kernel_tool_panics() {
        let mut reg = ToolRegistry::new();
        reg.register_kernel_tool(make_tool("dup", true));
        reg.register_kernel_tool(make_tool("dup", true));
    }
}
