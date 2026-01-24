//! Tool schema types and JSON Schema validation.

use serde::{Deserialize, Serialize};

use crate::error::CoreResult;
use crate::storage::EventLog;
use crate::workspace::Workspace;

/// Risk classification for a tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Tool has no side effects.
    Safe,
    /// Tool requires user confirmation before execution.
    Confirm,
    /// Tool is destructive and requires extra caution.
    Destructive,
}

/// Handler type: takes JSON args + mutable execution context, returns JSON result.
pub type ToolHandler =
    Box<dyn Fn(&serde_json::Value, &mut ExecutionContext) -> CoreResult<serde_json::Value> + Send + Sync>;

/// Mutable context passed to tool handlers.
pub struct ExecutionContext<'a> {
    pub workspace: &'a mut Workspace,
    pub event_log: &'a mut dyn EventLog,
}

/// Complete tool definition including schema, risk level, and handler.
pub struct ToolDefinition {
    /// Unique identifier for this tool.
    pub tool_id: String,
    /// JSON Schema for validating input arguments.
    pub input_schema: serde_json::Value,
    /// JSON Schema describing the output (documentation only in v0).
    pub output_schema: serde_json::Value,
    /// Risk classification.
    pub risk_level: RiskLevel,
    /// Whether this is a kernel-level tool (vs instance-scoped).
    pub is_kernel: bool,
    /// The handler function to execute.
    pub handler: ToolHandler,
}

/// Validate a JSON value against a minimal JSON Schema subset.
///
/// Supports: `type`, `required`, `properties` (recursive).
/// An empty schema `{}` passes anything.
pub fn validate_schema(value: &serde_json::Value, schema: &serde_json::Value) -> CoreResult<()> {
    // Empty schema passes anything
    let schema_obj = match schema.as_object() {
        Some(obj) => obj,
        None => return Ok(()),
    };

    if schema_obj.is_empty() {
        return Ok(());
    }

    // Check type constraint
    if let Some(type_val) = schema_obj.get("type") {
        let type_str = type_val
            .as_str()
            .ok_or_else(|| crate::error::CoreError::InvalidInput("schema 'type' must be a string".to_string()))?;

        let matches = match type_str {
            "string" => value.is_string(),
            "number" => value.is_number(),
            "integer" => value.is_i64() || value.is_u64(),
            "boolean" => value.is_boolean(),
            "object" => value.is_object(),
            "array" => value.is_array(),
            "null" => value.is_null(),
            other => {
                return Err(crate::error::CoreError::InvalidInput(format!(
                    "unknown schema type: {other}"
                )));
            }
        };

        if !matches {
            return Err(crate::error::CoreError::InvalidInput(format!(
                "expected type '{type_str}', got {}",
                json_type_name(value)
            )));
        }
    }

    // Check required fields (only meaningful for objects)
    if let Some(required) = schema_obj.get("required") {
        if let Some(required_arr) = required.as_array() {
            if let Some(obj) = value.as_object() {
                for req in required_arr {
                    if let Some(key) = req.as_str() {
                        if !obj.contains_key(key) {
                            return Err(crate::error::CoreError::InvalidInput(format!(
                                "missing required field: '{key}'"
                            )));
                        }
                    }
                }
            }
        }
    }

    // Recursively validate properties
    if let Some(properties) = schema_obj.get("properties") {
        if let (Some(props_obj), Some(val_obj)) = (properties.as_object(), value.as_object()) {
            for (key, prop_schema) in props_obj {
                if let Some(prop_value) = val_obj.get(key) {
                    validate_schema(prop_value, prop_schema)?;
                }
            }
        }
    }

    Ok(())
}

/// Returns a human-readable name for the JSON type of a value.
fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "integer"
            } else {
                "number"
            }
        }
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- Type checking tests ---

    #[test]
    fn validate_string_pass() {
        let schema = json!({"type": "string"});
        assert!(validate_schema(&json!("hello"), &schema).is_ok());
    }

    #[test]
    fn validate_string_fail() {
        let schema = json!({"type": "string"});
        assert!(validate_schema(&json!(42), &schema).is_err());
    }

    #[test]
    fn validate_number_pass() {
        let schema = json!({"type": "number"});
        assert!(validate_schema(&json!(3.14), &schema).is_ok());
        assert!(validate_schema(&json!(42), &schema).is_ok());
    }

    #[test]
    fn validate_number_fail() {
        let schema = json!({"type": "number"});
        assert!(validate_schema(&json!("not a number"), &schema).is_err());
    }

    #[test]
    fn validate_integer_pass() {
        let schema = json!({"type": "integer"});
        assert!(validate_schema(&json!(42), &schema).is_ok());
    }

    #[test]
    fn validate_integer_fail_float() {
        let schema = json!({"type": "integer"});
        assert!(validate_schema(&json!(3.14), &schema).is_err());
    }

    #[test]
    fn validate_boolean_pass() {
        let schema = json!({"type": "boolean"});
        assert!(validate_schema(&json!(true), &schema).is_ok());
        assert!(validate_schema(&json!(false), &schema).is_ok());
    }

    #[test]
    fn validate_boolean_fail() {
        let schema = json!({"type": "boolean"});
        assert!(validate_schema(&json!(1), &schema).is_err());
    }

    #[test]
    fn validate_array_pass() {
        let schema = json!({"type": "array"});
        assert!(validate_schema(&json!([1, 2, 3]), &schema).is_ok());
    }

    #[test]
    fn validate_array_fail() {
        let schema = json!({"type": "array"});
        assert!(validate_schema(&json!({}), &schema).is_err());
    }

    #[test]
    fn validate_null_pass() {
        let schema = json!({"type": "null"});
        assert!(validate_schema(&serde_json::Value::Null, &schema).is_ok());
    }

    #[test]
    fn validate_null_fail() {
        let schema = json!({"type": "null"});
        assert!(validate_schema(&json!("not null"), &schema).is_err());
    }

    #[test]
    fn validate_object_pass() {
        let schema = json!({"type": "object"});
        assert!(validate_schema(&json!({"key": "value"}), &schema).is_ok());
    }

    #[test]
    fn validate_object_fail() {
        let schema = json!({"type": "object"});
        assert!(validate_schema(&json!([]), &schema).is_err());
    }

    // --- Required fields tests ---

    #[test]
    fn validate_required_pass() {
        let schema = json!({
            "type": "object",
            "required": ["name", "age"]
        });
        let value = json!({"name": "Alice", "age": 30});
        assert!(validate_schema(&value, &schema).is_ok());
    }

    #[test]
    fn validate_required_fail() {
        let schema = json!({
            "type": "object",
            "required": ["name", "age"]
        });
        let value = json!({"name": "Alice"});
        assert!(validate_schema(&value, &schema).is_err());
    }

    // --- Recursive properties validation ---

    #[test]
    fn validate_properties_recursive_pass() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "count": {"type": "integer"}
            }
        });
        let value = json!({"name": "test", "count": 5});
        assert!(validate_schema(&value, &schema).is_ok());
    }

    #[test]
    fn validate_properties_recursive_fail() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "count": {"type": "integer"}
            }
        });
        let value = json!({"name": "test", "count": "not an integer"});
        assert!(validate_schema(&value, &schema).is_err());
    }

    // --- Empty schema ---

    #[test]
    fn empty_schema_passes_anything() {
        let schema = json!({});
        assert!(validate_schema(&json!("string"), &schema).is_ok());
        assert!(validate_schema(&json!(42), &schema).is_ok());
        assert!(validate_schema(&json!(null), &schema).is_ok());
        assert!(validate_schema(&json!([1, 2]), &schema).is_ok());
        assert!(validate_schema(&json!({"key": "val"}), &schema).is_ok());
    }

    // --- RiskLevel serde roundtrip ---

    #[test]
    fn risk_level_serde_roundtrip() {
        for level in [RiskLevel::Safe, RiskLevel::Confirm, RiskLevel::Destructive] {
            let json = serde_json::to_string(&level).unwrap();
            let deserialized: RiskLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, level);
        }
    }
}
