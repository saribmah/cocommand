use std::collections::{HashSet, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use llm_kit_core::tool::ToolSet;
use llm_kit_provider_utils::tool::{Tool, ToolExecuteOptions, ToolExecutionOutput};
use serde_json::json;
use uuid::Uuid;

use crate::permissions::PermissionStore;
use crate::storage::Storage;
use crate::tools::executor::{execute_tool, ToolExecutionOutcome};
use crate::tools::registry::ToolRegistry;
use crate::workspace::Workspace;

/// Shared runtime state required to execute tools via llm-kit.
pub struct ToolRuntime {
    pub registry: Arc<Mutex<ToolRegistry>>,
    pub workspace: Arc<Mutex<Workspace>>,
    pub storage: Arc<Mutex<Box<dyn Storage>>>,
    pub permission_store: Arc<Mutex<PermissionStore>>,
    pub instance_id: String,
}

/// Build a llm-kit ToolSet backed by the cocommand ToolRegistry.
pub fn build_toolset(runtime: Arc<ToolRuntime>) -> ToolSet {
    let descriptors = {
        let registry = runtime.registry.lock().expect("registry lock");
        registry
            .kernel_tools()
            .into_iter()
            .map(|(id, def)| {
                (
                    id.to_string(),
                    def.input_schema.clone(),
                    def.output_schema.clone(),
                )
            })
            .collect::<Vec<_>>()
    };

    let mut tools = ToolSet::new();
    let mut used_names = HashSet::new();

    for (tool_id, input_schema, output_schema) in descriptors {
        let runtime = Arc::clone(&runtime);
        let tool_id_for_exec = tool_id.clone();
        let safe_tool_name = sanitize_tool_name(&tool_id, &mut used_names);
        let input_schema = normalize_input_schema(input_schema);

        let tool = Tool::function(input_schema)
            .with_output_schema(output_schema)
            .with_execute(Arc::new(move |input, options: ToolExecuteOptions| {
                let runtime = Arc::clone(&runtime);
                let tool_id = tool_id_for_exec.clone();

                ToolExecutionOutput::Single(Box::pin(async move {
                    let tool_call_id = Uuid::parse_str(&options.tool_call_id)
                        .unwrap_or_else(|_| Uuid::new_v4());

                    let registry = runtime.registry.lock().expect("registry lock");
                    let mut workspace = runtime.workspace.lock().expect("workspace lock");
                    let mut storage = runtime.storage.lock().expect("storage lock");
                    let permission_store =
                        runtime.permission_store.lock().expect("permission store lock");
                    let (event_log, clipboard_store) = storage.split_event_clipboard_mut();

                    match execute_tool(
                        &registry,
                        &mut workspace,
                        event_log,
                        clipboard_store,
                        &permission_store,
                        &runtime.instance_id,
                        &tool_id,
                        input,
                        tool_call_id,
                    ) {
                        ToolExecutionOutcome::Executed(exec) => Ok(exec.result),
                        ToolExecutionOutcome::NeedsConfirmation { confirmation_id } => Err(json!({
                            "type": "approval_required",
                            "confirmation_id": confirmation_id,
                            "tool_id": tool_id,
                        })),
                        ToolExecutionOutcome::Denied { reason, .. } => Err(json!({
                            "type": "tool_denied",
                            "tool_id": tool_id,
                            "reason": reason,
                        })),
                    }
                }))
            }));

        if safe_tool_name != tool_id {
            println!(
                "[planner] tool_name_mapped original={} safe={}",
                tool_id, safe_tool_name
            );
        }
        tools.insert(safe_tool_name, tool);
    }

    tools
}

fn normalize_input_schema(schema: serde_json::Value) -> serde_json::Value {
    let mut obj = match schema.as_object() {
        Some(map) => map.clone(),
        None => return serde_json::json!({ "type": "object", "properties": {} }),
    };

    if obj.is_empty() {
        return serde_json::json!({ "type": "object", "properties": {} });
    }

    obj.entry("type".to_string())
        .or_insert_with(|| serde_json::Value::String("object".to_string()));
    obj.entry("properties".to_string())
        .or_insert_with(|| serde_json::json!({}));

    serde_json::Value::Object(obj)
}

fn sanitize_tool_name(original: &str, used: &mut HashSet<String>) -> String {
    let mut safe: String = original
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' => ch,
            _ => '_',
        })
        .collect();

    if safe.is_empty() {
        safe = "tool".to_string();
    }

    if used.insert(safe.clone()) {
        return safe;
    }

    let suffix = short_hash_suffix(original);
    let candidate = format!("{safe}_{suffix}");
    if used.insert(candidate.clone()) {
        candidate
    } else {
        let fallback = format!("{safe}_{}", used.len());
        used.insert(fallback.clone());
        fallback
    }
}

fn short_hash_suffix(value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:x}", hasher.finish())
        .chars()
        .take(8)
        .collect()
}
