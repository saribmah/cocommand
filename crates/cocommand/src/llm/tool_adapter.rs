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

    for (tool_id, input_schema, output_schema) in descriptors {
        let runtime = Arc::clone(&runtime);
        let tool_id_for_exec = tool_id.clone();

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

        tools.insert(tool_id, tool);
    }

    tools
}
