use cocommand_llm::{LlmProvider, LlmStreamEvent, LlmToolSet};
use tokio_stream::StreamExt;

use crate::error::{CoreError, CoreResult};
use crate::extension::ExtensionContext;
use crate::llm::settings_from_workspace;
use crate::storage::SharedStorage;
use crate::tool::registry::{build_tool, sanitize_tool_name};
use crate::utils::time::now_secs;

use super::types::{Agent, AgentSummary, ExecuteAgentPayload, ListAgentsPayload};

const STORAGE_NAMESPACE: &str = "agents";

pub async fn list_agents(storage: &SharedStorage) -> CoreResult<ListAgentsPayload> {
    let ids = storage.list(&[STORAGE_NAMESPACE]).await?;
    let mut agents: Vec<AgentSummary> = Vec::new();

    for id in ids {
        if let Some(value) = storage.read(&[STORAGE_NAMESPACE, &id]).await? {
            let agent: Agent = serde_json::from_value(value)
                .map_err(|e| CoreError::Internal(format!("failed to parse agent {id}: {e}")))?;
            agents.push(AgentSummary::from(&agent));
        }
    }

    agents.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    let count = agents.len();
    Ok(ListAgentsPayload { agents, count })
}

pub async fn create_agent(
    storage: &SharedStorage,
    name: String,
    description: Option<String>,
    personality: String,
    memory: Option<String>,
    extensions: Option<Vec<String>>,
) -> CoreResult<Agent> {
    let base_id = slugify(&name);
    let id = ensure_unique_id(storage, &base_id).await?;
    let now = now_secs();

    let agent = Agent {
        id: id.clone(),
        name,
        description: description.unwrap_or_default(),
        personality,
        memory: memory.unwrap_or_default(),
        extensions: extensions.unwrap_or_default(),
        created_at: now,
        updated_at: now,
    };

    let value = serde_json::to_value(&agent)
        .map_err(|e| CoreError::Internal(format!("failed to serialize agent: {e}")))?;
    storage.write(&[STORAGE_NAMESPACE, &id], &value).await?;
    Ok(agent)
}

pub async fn get_agent(storage: &SharedStorage, id: &str) -> CoreResult<Agent> {
    let value = storage
        .read(&[STORAGE_NAMESPACE, id])
        .await?
        .ok_or_else(|| CoreError::InvalidInput(format!("agent not found: {id}")))?;
    serde_json::from_value(value)
        .map_err(|e| CoreError::Internal(format!("failed to parse agent {id}: {e}")))
}

pub async fn update_agent(
    storage: &SharedStorage,
    id: &str,
    name: Option<String>,
    description: Option<String>,
    personality: Option<String>,
    memory: Option<String>,
    extensions: Option<Vec<String>>,
) -> CoreResult<Agent> {
    let mut agent = get_agent(storage, id).await?;

    if let Some(name) = name {
        agent.name = name;
    }
    if let Some(description) = description {
        agent.description = description;
    }
    if let Some(personality) = personality {
        agent.personality = personality;
    }
    if let Some(memory) = memory {
        agent.memory = memory;
    }
    if let Some(extensions) = extensions {
        agent.extensions = extensions;
    }
    agent.updated_at = now_secs();

    let value = serde_json::to_value(&agent)
        .map_err(|e| CoreError::Internal(format!("failed to serialize agent: {e}")))?;
    storage.write(&[STORAGE_NAMESPACE, id], &value).await?;
    Ok(agent)
}

pub async fn delete_agent(storage: &SharedStorage, id: &str) -> CoreResult<bool> {
    let exists = storage.read(&[STORAGE_NAMESPACE, id]).await?.is_some();
    if !exists {
        return Ok(false);
    }
    storage.delete(&[STORAGE_NAMESPACE, id]).await?;
    Ok(true)
}

pub async fn execute_agent(
    context: &ExtensionContext,
    llm: &dyn LlmProvider,
    id: &str,
    message: &str,
) -> CoreResult<ExecuteAgentPayload> {
    let storage = &context.workspace.storage;
    let agent = get_agent(storage, id).await?;

    // Build agent system prompt
    let mut system_prompt = agent.personality.clone();
    if !agent.memory.is_empty() {
        system_prompt.push_str("\n\n## Memory\n\n");
        system_prompt.push_str(&agent.memory);
    }

    // Build LLM settings from workspace config, override system prompt
    let settings = {
        let config = context.workspace.config.read().await;
        let mut settings = settings_from_workspace(&config.llm);
        settings.system_prompt = system_prompt;
        settings
    };

    let agent_llm = llm
        .with_settings(settings)
        .map_err(|e| CoreError::Internal(format!("failed to create agent provider: {e}")))?;

    // Build constrained tool set from agent's allowed extensions
    let mut tool_set = LlmToolSet::new();
    {
        let registry = context.workspace.extension_registry.read().await;
        let ext_context = context.clone();
        for ext_id in &agent.extensions {
            if let Some(ext) = registry.get(ext_id) {
                for tool in ext.tools() {
                    let raw_name = format!("{}.{}", ext_id, tool.id);
                    let tool_name = sanitize_tool_name(&raw_name);
                    let built = build_tool(tool, ext_context.clone());
                    tool_set.insert(tool_name, built);
                }
            }
        }
    }

    // Build user message
    let user_message = cocommand_llm::Message::from_text("agent", "user", message);
    let messages = vec![user_message];

    let stream = agent_llm
        .stream(&messages, tool_set)
        .await
        .map_err(|e| CoreError::Internal(format!("agent execution failed: {e}")))?;

    // Consume stream, collecting text deltas
    let mut response = String::new();
    let mut pinned_stream = stream;
    while let Some(part) = pinned_stream.next().await {
        match part {
            LlmStreamEvent::TextDelta { text, .. } => {
                response.push_str(&text);
            }
            LlmStreamEvent::Error { error } => {
                return Err(CoreError::Internal(format!("agent stream error: {error}")));
            }
            _ => {}
        }
    }

    Ok(ExecuteAgentPayload {
        agent_id: agent.id,
        agent_name: agent.name,
        response,
    })
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for ch in input.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if !slug.is_empty() && !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "agent".to_string()
    } else {
        slug
    }
}

async fn ensure_unique_id(storage: &SharedStorage, base_id: &str) -> CoreResult<String> {
    let existing = storage.list(&[STORAGE_NAMESPACE]).await?;
    if !existing.contains(&base_id.to_string()) {
        return Ok(base_id.to_string());
    }
    let mut suffix = 1usize;
    loop {
        let candidate = format!("{base_id}-{suffix}");
        if !existing.contains(&candidate) {
            return Ok(candidate);
        }
        suffix += 1;
    }
}
