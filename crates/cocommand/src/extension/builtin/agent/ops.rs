use std::sync::Arc;

use crate::command::runtime::SessionRuntimeRegistry;
use crate::command::session_message::{SessionCommandInputPart, SessionCommandTextPartInput};
use crate::error::{CoreError, CoreResult};
use crate::extension::ExtensionContext;
use crate::session::SessionManager;
use crate::storage::SharedStorage;
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
    runtime_registry: &SessionRuntimeRegistry,
    id: &str,
    message: &str,
) -> CoreResult<ExecuteAgentPayload> {
    let storage = &context.workspace.storage;
    let agent = get_agent(storage, id).await?;
    let child_sessions = Arc::new(SessionManager::new(context.workspace.clone()));

    let allowed_extensions = {
        let registry = context.workspace.extension_registry.read().await;
        let mut extensions = Vec::new();
        for extension_id in &agent.extensions {
            if let Some(extension) = registry.get(extension_id) {
                extensions.push((extension_id.clone(), extension));
                continue;
            }
            tracing::warn!(
                "skipping missing extension {} while executing agent {}",
                extension_id,
                agent.id
            );
        }
        extensions
    };

    let child_session_id = child_sessions
        .with_fresh_session_mut(|session| {
            let workspace = context.workspace.clone();
            let allowed_extensions = allowed_extensions.clone();
            Box::pin(async move {
                let session_id = session.session_id.clone();
                let extension_context = ExtensionContext {
                    workspace,
                    session_id: session_id.clone(),
                };

                for (extension_id, extension) in &allowed_extensions {
                    extension.activate(&extension_context).await?;
                    session.activate_extension(extension_id);
                }

                Ok(session_id)
            })
        })
        .await?;

    let runtime = runtime_registry
        .spawn_with_session_manager(child_session_id.clone(), child_sessions)
        .await;

    let ack = runtime
        .enqueue_user_message(vec![SessionCommandInputPart::Text(
            SessionCommandTextPartInput {
                text: message.to_string(),
            },
        )])
        .await?;

    Ok(ExecuteAgentPayload {
        agent_id: agent.id,
        agent_name: agent.name,
        session_id: child_session_id,
        run_id: ack.run_id,
        status: "queued".to_string(),
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use futures_util::stream;
    use serde_json::json;
    use tempfile::tempdir;

    use super::*;
    use crate::bus::Bus;
    use crate::command::runtime::SessionRuntimeRegistry;
    use crate::llm::{
        LlmError, LlmProvider, LlmSettings, LlmStream, LlmStreamEvent, LlmStreamOptions,
    };
    use crate::message::{Message, MessagePart};
    use crate::session::SessionManager;
    use crate::workspace::WorkspaceInstance;

    #[derive(Clone)]
    struct FakeLlmProvider;

    #[async_trait]
    impl LlmProvider for FakeLlmProvider {
        async fn stream(
            &self,
            _messages: &[Message],
            _tools: crate::llm::LlmToolSet,
        ) -> Result<LlmStream, LlmError> {
            self.stream_with_options(
                &[],
                crate::llm::LlmToolSet::new(),
                LlmStreamOptions::default(),
            )
            .await
        }

        async fn stream_with_options(
            &self,
            _messages: &[Message],
            _tools: crate::llm::LlmToolSet,
            _options: LlmStreamOptions,
        ) -> Result<LlmStream, LlmError> {
            Ok(Box::pin(stream::iter(vec![LlmStreamEvent::Finish])))
        }

        async fn update_settings(&self, _settings: LlmSettings) -> Result<(), LlmError> {
            Ok(())
        }

        fn with_settings(&self, _settings: LlmSettings) -> Result<Box<dyn LlmProvider>, LlmError> {
            Ok(Box::new(self.clone()))
        }
    }

    #[tokio::test]
    async fn execute_agent_returns_queued_ack_and_enqueues_message() {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::new(dir.path()).await.expect("workspace"));

        let agent = create_agent(
            &workspace.storage,
            "Queue Agent".to_string(),
            Some("description".to_string()),
            "personality".to_string(),
            Some("memory".to_string()),
            None,
        )
        .await
        .expect("create agent");

        let runtime_registry = SessionRuntimeRegistry::new(
            workspace.as_ref().clone(),
            Arc::new(SessionManager::new(workspace.clone())),
            Arc::new(FakeLlmProvider),
            Bus::new(64),
        );

        let payload = execute_agent(
            &ExtensionContext {
                workspace: workspace.clone(),
                session_id: "parent-session".to_string(),
            },
            &runtime_registry,
            &agent.id,
            "child task input",
        )
        .await
        .expect("execute agent");

        assert_eq!(payload.agent_id, agent.id);
        assert_eq!(payload.agent_name, agent.name);
        assert_eq!(payload.status, "queued");
        assert!(!payload.session_id.is_empty());
        assert!(!payload.run_id.is_empty());

        let messages =
            crate::message::message::MessageStorage::load(&workspace.storage, &payload.session_id)
                .await
                .expect("load child messages");
        let user_message = messages
            .iter()
            .find(|message| message.info.role == "user")
            .expect("user message");
        assert!(user_message.parts.iter().any(|part| matches!(
            part,
            MessagePart::Text(text) if text.text == "child task input"
        )));
    }

    #[tokio::test]
    async fn execute_agent_skips_missing_extensions() {
        let dir = tempdir().expect("tempdir");
        let workspace = Arc::new(WorkspaceInstance::new(dir.path()).await.expect("workspace"));

        let agent = create_agent(
            &workspace.storage,
            "Missing Extension Agent".to_string(),
            None,
            "personality".to_string(),
            None,
            Some(vec!["not-installed".to_string()]),
        )
        .await
        .expect("create agent");

        let runtime_registry = SessionRuntimeRegistry::new(
            workspace.as_ref().clone(),
            Arc::new(SessionManager::new(workspace.clone())),
            Arc::new(FakeLlmProvider),
            Bus::new(64),
        );

        let payload = execute_agent(
            &ExtensionContext {
                workspace: workspace.clone(),
                session_id: "parent-session".to_string(),
            },
            &runtime_registry,
            &agent.id,
            "run anyway",
        )
        .await
        .expect("execute agent");

        assert_eq!(payload.status, "queued");
        let messages =
            crate::message::message::MessageStorage::load(&workspace.storage, &payload.session_id)
                .await
                .expect("load child messages");
        assert!(messages.iter().any(|message| message.info.role == "user"));
    }

    #[test]
    fn slugify_generates_expected_ids() {
        assert_eq!(slugify("My Agent"), "my-agent");
        assert_eq!(slugify("  ???  "), "agent");
        assert_eq!(slugify("a___b"), "a-b");
    }

    #[tokio::test]
    async fn ensure_unique_id_appends_suffix_when_needed() {
        let dir = tempdir().expect("tempdir");
        let workspace = WorkspaceInstance::new(dir.path()).await.expect("workspace");
        let storage = workspace.storage;
        storage
            .write(&[STORAGE_NAMESPACE, "agent"], &json!({"id": "agent"}))
            .await
            .expect("seed");
        storage
            .write(&[STORAGE_NAMESPACE, "agent-1"], &json!({"id": "agent-1"}))
            .await
            .expect("seed");
        let id = ensure_unique_id(&storage, "agent")
            .await
            .expect("unique id");
        assert_eq!(id, "agent-2");
    }
}
