use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;

use crate::error::{CoreError, CoreResult};
use crate::extension::builtin::manifest_tools::{merge_manifest_tools, parse_builtin_manifest};
use crate::extension::manifest::ExtensionManifest;
use crate::extension::{
    boxed_tool_future, Extension, ExtensionInitContext, ExtensionKind, ExtensionStatus,
    ExtensionTool,
};
use crate::llm::LlmProvider;

use super::ops;

pub struct AgentExtension {
    manifest: ExtensionManifest,
    tools: Vec<ExtensionTool>,
}

impl std::fmt::Debug for AgentExtension {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("AgentExtension").finish()
    }
}

impl AgentExtension {
    pub fn new(llm: Arc<dyn LlmProvider>) -> Self {
        let manifest = parse_builtin_manifest(include_str!("manifest.json"));
        let mut execute_map = HashMap::new();

        execute_map.insert(
            "list-agents",
            std::sync::Arc::new(
                |_input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let payload = ops::list_agents(&context.workspace.storage).await?;
                        Ok(json!(payload))
                    })
                },
            ) as _,
        );

        execute_map.insert(
            "create-agent",
            std::sync::Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let name = required_string(&input, "name")?;
                        let personality = required_string(&input, "personality")?;
                        let description = optional_string(&input, "description");
                        let memory = optional_string(&input, "memory");
                        let extensions = optional_string_array(&input, "extensions");
                        let agent = ops::create_agent(
                            &context.workspace.storage,
                            name,
                            description,
                            personality,
                            memory,
                            extensions,
                        )
                        .await?;
                        Ok(json!(agent))
                    })
                },
            ) as _,
        );

        execute_map.insert(
            "get-agent",
            std::sync::Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let id = required_string(&input, "id")?;
                        let agent = ops::get_agent(&context.workspace.storage, &id).await?;
                        Ok(json!(agent))
                    })
                },
            ) as _,
        );

        execute_map.insert(
            "update-agent",
            std::sync::Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let id = required_string(&input, "id")?;
                        let name = optional_string(&input, "name");
                        let description = optional_string(&input, "description");
                        let personality = optional_string(&input, "personality");
                        let memory = optional_string(&input, "memory");
                        let extensions = optional_string_array(&input, "extensions");
                        let agent = ops::update_agent(
                            &context.workspace.storage,
                            &id,
                            name,
                            description,
                            personality,
                            memory,
                            extensions,
                        )
                        .await?;
                        Ok(json!(agent))
                    })
                },
            ) as _,
        );

        execute_map.insert(
            "delete-agent",
            std::sync::Arc::new(
                |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                    boxed_tool_future(async move {
                        let id = required_string(&input, "id")?;
                        let deleted = ops::delete_agent(&context.workspace.storage, &id).await?;
                        Ok(json!({
                            "status": "ok",
                            "deleted": deleted,
                        }))
                    })
                },
            ) as _,
        );

        {
            let llm = llm.clone();
            execute_map.insert(
                "execute-agent",
                std::sync::Arc::new(
                    move |input: serde_json::Value, context: crate::extension::ExtensionContext| {
                        let llm = llm.clone();
                        boxed_tool_future(async move {
                            let id = required_string(&input, "id")?;
                            let message = required_string(&input, "message")?;
                            let payload =
                                ops::execute_agent(&context, llm.as_ref(), &id, &message).await?;
                            Ok(json!(payload))
                        })
                    },
                ) as _,
            );
        }

        let tools = merge_manifest_tools(&manifest, execute_map);

        Self { manifest, tools }
    }
}

#[async_trait::async_trait]
impl Extension for AgentExtension {
    fn id(&self) -> &str {
        &self.manifest.id
    }

    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn kind(&self) -> ExtensionKind {
        ExtensionKind::System
    }

    fn tags(&self) -> Vec<String> {
        self.manifest
            .routing
            .as_ref()
            .and_then(|r| r.keywords.clone())
            .unwrap_or_default()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn status(&self) -> ExtensionStatus {
        ExtensionStatus::Ready
    }

    async fn initialize(&self, _context: ExtensionInitContext) -> CoreResult<()> {
        Ok(())
    }

    fn tools(&self) -> Vec<ExtensionTool> {
        self.tools.clone()
    }
}

fn optional_string(input: &serde_json::Value, key: &str) -> Option<String> {
    input
        .get(key)
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn required_string(input: &serde_json::Value, key: &str) -> CoreResult<String> {
    let value = input
        .get(key)
        .and_then(|raw| raw.as_str())
        .ok_or_else(|| CoreError::InvalidInput(format!("missing {key}")))?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(CoreError::InvalidInput(format!("missing {key}")));
    }
    Ok(trimmed.to_string())
}

fn optional_string_array(input: &serde_json::Value, key: &str) -> Option<Vec<String>> {
    input.get(key).and_then(|value| {
        value.as_array().map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect()
        })
    })
}
