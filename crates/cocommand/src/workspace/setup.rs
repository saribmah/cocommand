use crate::error::CoreResult;
use crate::extension::builtin::agent::ops::create_agent;

use super::instance::WorkspaceInstance;

/// Builtin extension IDs the default Cocommand agent can access.
const DEFAULT_AGENT_EXTENSIONS: &[&str] = &[
    "notes",
    "filesystem",
    "clipboard",
    "system",
    "screenshot",
    "agent",
    "browser",
    "web",
    "terminal",
    "editor",
];

/// Ensures the workspace has all expected seed data.
///
/// Each step is idempotent — checks storage before writing.
/// Runs on every workspace init; skips work that already exists.
pub async fn run_workspace_setup(instance: &WorkspaceInstance) -> CoreResult<()> {
    setup_main_agent(instance).await?;
    Ok(())
}

/// Creates the default "Cocommand" agent with access to all builtin extensions.
/// Skips creation if an agent with id "cocommand" already exists.
async fn setup_main_agent(instance: &WorkspaceInstance) -> CoreResult<()> {
    let existing = instance.storage.list(&["agents"]).await?;
    if existing.iter().any(|id| id == "cocommand") {
        return Ok(());
    }

    let extensions: Vec<String> = DEFAULT_AGENT_EXTENSIONS
        .iter()
        .map(|s| s.to_string())
        .collect();

    create_agent(
        &instance.storage,
        "Cocommand".to_string(),
        Some("The default workspace agent with access to all builtin tools.".to_string()),
        "You are Cocommand, a helpful AI assistant. You help users manage their workspace, files, notes, clipboard, and system. Be concise and action-oriented.".to_string(),
        None,
        Some(extensions),
    )
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_instance(dir: &std::path::Path) -> WorkspaceInstance {
        WorkspaceInstance::new(dir)
            .await
            .expect("workspace instance")
    }

    #[tokio::test]
    async fn setup_creates_agent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let instance = test_instance(dir.path()).await;

        let agents = instance.storage.list(&["agents"]).await.expect("list");
        assert!(agents.contains(&"cocommand".to_string()));
    }

    #[tokio::test]
    async fn setup_is_idempotent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let instance = test_instance(dir.path()).await;

        // Run setup again — each step checks storage, so no duplicates
        run_workspace_setup(&instance).await.expect("second run");

        let agents = instance.storage.list(&["agents"]).await.expect("list");
        let cocommand_count = agents
            .iter()
            .filter(|id| id.starts_with("cocommand"))
            .count();
        assert_eq!(cocommand_count, 1);
    }
}
