use std::time::SystemTime;
use uuid::Uuid;

use crate::command;
use crate::error::CoreResult;
use crate::events::Event;
use crate::permissions::PermissionStore;
use crate::routing::Router;
use crate::storage::Storage;
use crate::types::{ActionSummary, ArtifactAction, CoreResponse, RoutedCandidate};
use crate::workspace::state::{Timestamp, WorkspaceMode};
use crate::workspace::{load_or_create_workspace_config, Workspace, WorkspaceConfig};
use crate::builtins;
use crate::planner::{Planner, PlannerError, PlannerInput, PlannerOutput, ToolSpec, StubPlanner};
use crate::tools::registry::ToolRegistry;
use crate::llm::{build_toolset, ToolRuntime};
use llm_kit_core::tool::ToolSet;
use std::sync::{Arc, Mutex};

/// Primary facade for the cocommand engine.
///
/// All orchestration flows are accessed through this struct.
/// Responses are returned as [`CoreResponse`] — the single stable shape
/// used across the Tauri boundary.
pub struct Core {
    workspace: Arc<Mutex<Workspace>>,
    workspace_config: WorkspaceConfig,
    router: Router,
    storage: Arc<Mutex<Box<dyn Storage>>>,
    registry: Arc<Mutex<ToolRegistry>>,
    permission_store: Arc<Mutex<PermissionStore>>,
    planner: Arc<dyn Planner>,
    planner_label: String,
}

impl Core {
    /// Create a new `Core` instance with the given storage backend.
    ///
    /// If the storage contains a previously saved workspace snapshot,
    /// the workspace is restored from it (with ephemeral state sanitized).
    /// Otherwise a fresh workspace is created.
    pub fn new(storage: Box<dyn Storage>) -> Self {
        let workspace_config = WorkspaceConfig::default_new();
        Self::new_with_config(storage, workspace_config)
    }

    /// Create a new `Core` instance with the given storage backend and workspace folder.
    pub fn new_with_workspace_dir(
        storage: Box<dyn Storage>,
        workspace_dir: &std::path::Path,
    ) -> CoreResult<Self> {
        let workspace_config = load_or_create_workspace_config(workspace_dir)?;
        Ok(Self::new_with_config(storage, workspace_config))
    }

    /// Create a new `Core` instance with the given storage backend and workspace config.
    pub fn new_with_config(storage: Box<dyn Storage>, workspace_config: WorkspaceConfig) -> Self {
        let storage = Arc::new(Mutex::new(storage));
        let snapshot = storage.lock().expect("storage lock").snapshots().load();

        let workspace = match snapshot {
            Some(snap) => match serde_json::from_value::<Workspace>(snap.data) {
                Ok(mut ws) => {
                    // Sanitize ephemeral state that must not survive a restart.
                    ws.confirmation_pending = None;
                    if ws.mode == WorkspaceMode::AwaitingConfirmation {
                        ws.mode = WorkspaceMode::Idle;
                    }

                    // Expire follow-up if TTL has passed or turns exhausted.
                    let now = Self::now();
                    if ws.follow_up.is_some() && !ws.is_follow_up_valid(now) {
                        ws.follow_up = None;
                        if ws.mode == WorkspaceMode::FollowUpActive {
                            ws.mode = WorkspaceMode::Idle;
                        }
                    }

                    ws
                }
                Err(_) => {
                    // Log a warning so corrupt snapshots are observable.
                    storage
                        .lock()
                        .expect("storage lock")
                        .event_log_mut()
                        .append(Event::ErrorRaised {
                        id: Uuid::new_v4(),
                        timestamp: SystemTime::now(),
                        code: "snapshot_corrupt".to_string(),
                        message: "Failed to deserialize workspace snapshot; starting fresh."
                            .to_string(),
                    });
                    Workspace::new(Uuid::new_v4().to_string())
                }
            },
            None => Workspace::new(Uuid::new_v4().to_string()),
        };

        Core {
            workspace: Arc::new(Mutex::new(workspace)),
            workspace_config,
            router: Router::new(),
            storage,
            registry: Arc::new(Mutex::new(ToolRegistry::new())),
            permission_store: Arc::new(Mutex::new(PermissionStore::new())),
            planner: Arc::new(StubPlanner),
            planner_label: "stub".to_string(),
        }
    }

    /// Create a `Core` with an existing workspace, router, and storage (for testing).
    pub fn with_state(workspace: Workspace, router: Router, storage: Box<dyn Storage>) -> Self {
        Core {
            workspace: Arc::new(Mutex::new(workspace)),
            workspace_config: WorkspaceConfig::default_new(),
            router,
            storage: Arc::new(Mutex::new(storage)),
            registry: Arc::new(Mutex::new(ToolRegistry::new())),
            permission_store: Arc::new(Mutex::new(PermissionStore::new())),
            planner: Arc::new(StubPlanner),
            planner_label: "stub".to_string(),
        }
    }

    /// Submit a natural-language command for processing.
    ///
    /// Returns a [`CoreResponse`] — either an `Artifact` with the routing
    /// result, or an `Error` if follow-up context has expired.
    /// Read-only verbs that produce a Preview response instead of an Artifact.
    const PREVIEW_VERBS: &'static [&'static str] = &["show", "view", "get", "display", "preview", "read"];

    pub fn submit_command(&mut self, text: &str) -> CoreResult<CoreResponse> {
        {
            let mut storage = self.storage.lock().expect("storage lock");
            storage.event_log_mut().append(Event::UserMessage {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            text: text.to_string(),
            });
        }

        let parsed = command::parse(text);
        let now = Self::now();

        // Check follow-up validity and route accordingly.
        let (follow_up_ctx, workspace_snapshot) = {
            let mut workspace = self.workspace.lock().expect("workspace lock");

            // If a confirmation is pending, the user must resolve it first.
            if let Some(cp) = &workspace.confirmation_pending {
                let response = CoreResponse::Confirmation {
                    confirmation_id: cp.confirmation_id.clone(),
                    prompt: format!("Pending action: {}", cp.tool_id),
                    description: "Please confirm or deny before submitting new commands.".to_string(),
                };
                let mut storage = self.storage.lock().expect("storage lock");
                self.save_snapshot_locked(&mut workspace, &mut *storage);
                return Ok(response);
            }

            let follow_up_ctx = if workspace.is_follow_up_valid(now) {
                workspace.follow_up.clone()
            } else if workspace.follow_up.is_some() {
                // TTL expired or turns exhausted — expire and return error.
                workspace.expire_follow_up();
                let mut storage = self.storage.lock().expect("storage lock");
                self.save_snapshot_locked(&mut workspace, &mut *storage);
                return Ok(CoreResponse::Error {
                    message: "Follow-up expired. Please provide the full command.".to_string(),
                });
            } else {
                None
            };

            // If we're in follow-up, consume a turn.
            if follow_up_ctx.is_some() {
                workspace.consume_follow_up_turn();
            }

            (follow_up_ctx, workspace.clone())
        };

        let routing_result = self
            .router
            .route_with_follow_up(&parsed, follow_up_ctx.as_ref());

        // Build the artifact content from routing candidates.
        let candidates = routing_result.candidates;
        let routed_candidates: Vec<RoutedCandidate> = candidates
            .iter()
            .map(|c| RoutedCandidate {
                app_id: c.app_id.clone(),
                score: c.score,
                explanation: c.explanation.clone(),
            })
            .collect();

        let is_preview = Self::is_preview_command(&parsed.normalized_text);
        let mut planner_error: Option<String> = None;
        let planner_output = if !candidates.is_empty() && !is_preview {
            let instance_id = workspace_snapshot
                .focus
                .clone()
                .unwrap_or_else(|| "kernel".to_string());
            println!(
                "[planner] using={} instance_id={} command={}",
                self.planner_label, instance_id, parsed.raw_text
            );
            let output = self.run_planner(PlannerInput {
                command: parsed.clone(),
                candidates: candidates.clone(),
                workspace: workspace_snapshot,
                tools: self.collect_tool_specs(),
                toolset: Some(self.build_llm_toolset(instance_id)),
            })
            .map(Some)
            .unwrap_or_else(|err| {
                println!("[planner] error={err:?}");
                let mut storage = self.storage.lock().expect("storage lock");
                storage.event_log_mut().append(Event::ErrorRaised {
                    id: Uuid::new_v4(),
                    timestamp: SystemTime::now(),
                    code: "planner_error".to_string(),
                    message: format!("{err:?}"),
                });
                planner_error = Some(format!("{err:?}"));
                None
            })
            .or_else(|| {
                println!("[planner] output=None");
                None
            });
            if output.is_some() {
                println!("[planner] output=Some");
            }
            output
        } else {
            None
        };

        if let Some(output) = &planner_output {
            println!(
                "[planner] done id={} steps={}",
                output.metadata.planner_id,
                output.plan.steps.len()
            );
            let mut storage = self.storage.lock().expect("storage lock");
            let event_log = storage.event_log_mut();
            for step in &output.plan.steps {
                event_log.append(Event::ToolCallProposed {
                    id: Uuid::new_v4(),
                    timestamp: SystemTime::now(),
                    tool_id: step.tool_id.clone(),
                    args: step.args.clone(),
                });
            }
        }

        {
            let mut workspace = self.workspace.lock().expect("workspace lock");
            if let Some(cp) = &workspace.confirmation_pending {
                let response = CoreResponse::Confirmation {
                    confirmation_id: cp.confirmation_id.clone(),
                    prompt: format!("Pending action: {}", cp.tool_id),
                    description: "Please confirm or deny before submitting new commands.".to_string(),
                };
                let mut storage = self.storage.lock().expect("storage lock");
                self.save_snapshot_locked(&mut workspace, &mut *storage);
                return Ok(response);
            }
        }

        let response = if candidates.is_empty() {
            CoreResponse::Artifact {
                content: "No matching capabilities found.".to_string(),
                actions: vec![],
            }
        } else if is_preview {
            let top = &routed_candidates[0];
            CoreResponse::Preview {
                title: format!("{} result", top.app_id),
                content: top.explanation.clone(),
            }
        } else {
            if let Some(message) = planner_error {
                CoreResponse::Error { message }
            } else
            if let Some(output) = &planner_output {
                println!(
                    "[planner] response_text_present={} tool_errors={}",
                    output
                        .response_text
                        .as_ref()
                        .map(|t| !t.trim().is_empty())
                        .unwrap_or(false),
                    output.tool_errors.len()
                );
                if let Some(error_msg) = planner_error_message(output) {
                    CoreResponse::Error { message: error_msg }
                } else if let Some(text) = output
                    .response_text
                    .as_ref()
                    .map(|t| t.trim())
                    .filter(|t| !t.is_empty())
                {
                    let workspace = self.workspace.lock().expect("workspace lock");
                    let actions = self.build_artifact_actions(&workspace);
                    CoreResponse::Artifact {
                        content: text.to_string(),
                        actions,
                    }
                } else {
                    let top = &routed_candidates[0];
                    let content = format!(
                        "Routed to {} (score: {:.1}): {}",
                        top.app_id, top.score, top.explanation
                    );
                    let workspace = self.workspace.lock().expect("workspace lock");
                    let actions = self.build_artifact_actions(&workspace);
                    CoreResponse::Artifact { content, actions }
                }
            } else {
                let top = &routed_candidates[0];
                let content = format!(
                    "Routed to {} (score: {:.1}): {}",
                    top.app_id, top.score, top.explanation
                );
                let workspace = self.workspace.lock().expect("workspace lock");
                let actions = self.build_artifact_actions(&workspace);
                CoreResponse::Artifact { content, actions }
            }
        };

        {
            let mut workspace = self.workspace.lock().expect("workspace lock");
            let mut storage = self.storage.lock().expect("storage lock");
            self.save_snapshot_locked(&mut workspace, &mut *storage);
        }
        Ok(response)
    }

    /// Activate follow-up mode after a successful command execution.
    pub fn activate_follow_up(
        &mut self,
        command: String,
        entity_ids: Vec<String>,
        app_id: String,
    ) {
        let mut workspace = self.workspace.lock().expect("workspace lock");
        workspace.enter_follow_up(command, entity_ids, app_id);
    }

    /// Confirm or deny a pending action.
    ///
    /// Looks up the pending confirmation in the workspace by ID.
    /// Returns an `Artifact` on success or an `Error` if no matching
    /// confirmation is found.
    pub fn confirm_action(
        &mut self,
        confirmation_id: &str,
        decision: bool,
    ) -> CoreResult<CoreResponse> {
        let mut workspace = self.workspace.lock().expect("workspace lock");
        let mut storage = self.storage.lock().expect("storage lock");
        let pending = workspace.confirmation_pending.as_ref();

        match pending {
            Some(cp) if cp.confirmation_id == confirmation_id => {
                let tool_id = cp.tool_id.clone();
                workspace.clear_confirmation();

                let response = if decision {
                    CoreResponse::Artifact {
                        content: format!("Action confirmed: {tool_id}"),
                        actions: vec![],
                    }
                } else {
                    CoreResponse::Artifact {
                        content: format!("Action cancelled: {tool_id}"),
                        actions: vec![],
                    }
                };

                self.save_snapshot_locked(&mut workspace, &mut *storage);
                Ok(response)
            }
            Some(_) => Ok(CoreResponse::Error {
                message: format!(
                    "No pending confirmation with id '{confirmation_id}'."
                ),
            }),
            None => Ok(CoreResponse::Error {
                message: "No action pending confirmation.".to_string(),
            }),
        }
    }

    /// Retrieve a snapshot of the current workspace state.
    pub fn get_workspace_snapshot(&self) -> CoreResult<Workspace> {
        Ok(self.workspace.lock().expect("workspace lock").clone())
    }

    /// Retrieve recent actions up to `limit`.
    ///
    /// Returns pre-computed summaries from the event log. Summaries are
    /// derived from structural metadata at write time and never contain
    /// raw user text or sensitive content.
    pub fn get_recent_actions(&self, limit: usize) -> CoreResult<Vec<ActionSummary>> {
        let storage = self.storage.lock().expect("storage lock");
        let records = storage.event_log().tail(limit);
        let summaries = records
            .into_iter()
            .map(|record| ActionSummary {
                id: record.event.id().to_string(),
                description: record.summary,
            })
            .collect();
        Ok(summaries)
    }

    /// Get a reference to the storage backend.
    pub fn storage(&self) -> std::sync::MutexGuard<'_, Box<dyn Storage>> {
        self.storage.lock().expect("storage lock")
    }

    /// Get a mutable reference to the storage backend.
    pub fn storage_mut(&mut self) -> std::sync::MutexGuard<'_, Box<dyn Storage>> {
        self.storage.lock().expect("storage lock")
    }

    /// Get a mutable reference to the router for registration.
    pub fn router_mut(&mut self) -> &mut Router {
        &mut self.router
    }

    /// Get a reference to the workspace.
    pub fn workspace(&self) -> Workspace {
        self.workspace.lock().expect("workspace lock").clone()
    }

    /// Get a copy of the workspace configuration.
    pub fn workspace_config(&self) -> WorkspaceConfig {
        self.workspace_config.clone()
    }

    /// Get a mutable reference to the workspace (for testing).
    #[cfg(test)]
    pub fn workspace_mut(&mut self) -> std::sync::MutexGuard<'_, Workspace> {
        self.workspace.lock().expect("workspace lock")
    }

    /// Build a tool runtime for llm-kit execution.
    pub fn tool_runtime(&self, instance_id: impl Into<String>) -> Arc<ToolRuntime> {
        Arc::new(ToolRuntime {
            registry: Arc::clone(&self.registry),
            workspace: Arc::clone(&self.workspace),
            storage: Arc::clone(&self.storage),
            permission_store: Arc::clone(&self.permission_store),
            instance_id: instance_id.into(),
        })
    }

    /// Build a llm-kit ToolSet wired to this core.
    pub fn build_llm_toolset(&self, instance_id: impl Into<String>) -> ToolSet {
        build_toolset(self.tool_runtime(instance_id))
    }

    /// Access the tool registry (for registration).
    pub fn registry_mut(&mut self) -> std::sync::MutexGuard<'_, ToolRegistry> {
        self.registry.lock().expect("registry lock")
    }

    /// Register built-in tools and router metadata.
    pub fn register_builtins(&mut self) {
        let mut registry = self.registry.lock().expect("registry lock");
        let router = &mut self.router;
        builtins::register_builtins(&mut *registry, router);
    }

    /// Access the permission store (for updates).
    pub fn permission_store_mut(&mut self) -> std::sync::MutexGuard<'_, PermissionStore> {
        self.permission_store.lock().expect("permission store lock")
    }

    /// Set a custom planner implementation.
    pub fn set_planner(&mut self, planner: Arc<dyn Planner>) {
        self.planner = planner;
        self.planner_label = "custom".to_string();
    }

    /// Set a custom planner with a label for logging.
    pub fn set_planner_with_label(&mut self, planner: Arc<dyn Planner>, label: impl Into<String>) {
        self.planner = planner;
        self.planner_label = label.into();
    }

    /// Get the current unix timestamp in seconds.
    fn now() -> Timestamp {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Check if the command text starts with a read-only verb.
    fn is_preview_command(text: &str) -> bool {
        let first_word = text.split_whitespace().next().unwrap_or("");
        Self::PREVIEW_VERBS
            .iter()
            .any(|v| first_word.eq_ignore_ascii_case(v))
    }

    fn collect_tool_specs(&self) -> Vec<ToolSpec> {
        let registry = self.registry.lock().expect("registry lock");
        registry
            .kernel_tools()
            .into_iter()
            .map(|(id, def)| ToolSpec {
                tool_id: id.to_string(),
                input_schema: def.input_schema.clone(),
                output_schema: def.output_schema.clone(),
                risk_level: def.risk_level.clone(),
                is_kernel: def.is_kernel,
            })
            .collect()
    }

    fn run_planner(&self, input: PlannerInput) -> Result<PlannerOutput, PlannerError> {
        let future = self.planner.plan(input);
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.block_on(future)
        } else {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build runtime")
                .block_on(future)
        }
    }

    /// Persist the current workspace state as a snapshot in storage.
    fn save_snapshot_locked(
        &self,
        workspace: &mut Workspace,
        storage: &mut Box<dyn Storage>,
    ) {
        if let Ok(data) = serde_json::to_value(&*workspace) {
            let snapshot = crate::storage::types::WorkspaceSnapshot {
                session_id: workspace.session_id.clone(),
                captured_at: SystemTime::now(),
                data,
            };
            storage.snapshots_mut().save(snapshot);
        }
    }

    /// Build artifact actions based on the current workspace mode.
    fn build_artifact_actions(&self, workspace: &Workspace) -> Vec<ArtifactAction> {
        if let Some(cp) = &workspace.confirmation_pending {
            vec![ArtifactAction {
                id: cp.confirmation_id.clone(),
                label: format!("Confirm: {}", cp.tool_id),
            }]
        } else {
            vec![]
        }
    }
}

fn planner_error_message(output: &PlannerOutput) -> Option<String> {
    let error = output.tool_errors.first()?;
    let error_type = error
        .as_object()
        .and_then(|obj| obj.get("type"))
        .and_then(|value| value.as_str());

    match error_type {
        Some("tool_denied") => error
            .as_object()
            .and_then(|obj| obj.get("reason"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .or_else(|| Some("Tool execution denied.".to_string())),
        Some("approval_required") => Some("Approval required.".to_string()),
        _ => Some("Tool execution failed.".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::RoutingMetadata;
    use crate::storage::MemoryStorage;
    use crate::workspace::state::{
        ConfirmationPending, FollowUpContext, WorkspaceMode, FOLLOW_UP_MAX_TURNS,
    };

    fn make_storage() -> Box<dyn Storage> {
        Box::new(MemoryStorage::new())
    }

    fn calendar_metadata() -> RoutingMetadata {
        RoutingMetadata {
            app_id: "calendar".to_string(),
            keywords: vec!["schedule".into(), "meeting".into(), "event".into()],
            examples: vec!["schedule a meeting".into(), "create an event".into()],
            verbs: vec!["schedule".into(), "create".into(), "cancel".into()],
            objects: vec!["meeting".into(), "event".into(), "appointment".into()],
        }
    }

    fn notes_metadata() -> RoutingMetadata {
        RoutingMetadata {
            app_id: "notes".to_string(),
            keywords: vec!["note".into(), "memo".into()],
            examples: vec!["show last note".into()],
            verbs: vec!["show".into(), "write".into(), "create".into()],
            objects: vec!["note".into(), "memo".into()],
        }
    }

    #[test]
    fn core_new_returns_instance() {
        let _core = Core::new(make_storage());
    }

    #[test]
    fn submit_command_returns_artifact() {
        let mut core = Core::new(make_storage());
        core.router_mut().register(calendar_metadata());

        let resp = core.submit_command("schedule a meeting").unwrap();
        match resp {
            CoreResponse::Artifact { content, actions } => {
                assert!(content.contains("calendar"));
                assert!(actions.is_empty());
            }
            _ => panic!("Expected Artifact response"),
        }
    }

    #[test]
    fn submit_command_no_candidates_returns_artifact() {
        let mut core = Core::new(make_storage());
        // No apps registered — should still return Artifact with "no match" message.
        let resp = core.submit_command("do something unknown").unwrap();
        match resp {
            CoreResponse::Artifact { content, .. } => {
                assert!(content.contains("No matching"));
            }
            _ => panic!("Expected Artifact response"),
        }
    }

    #[test]
    fn submit_command_preview_verb_returns_preview() {
        let mut core = Core::new(make_storage());
        core.router_mut().register(notes_metadata());

        let resp = core.submit_command("show last note").unwrap();
        match resp {
            CoreResponse::Preview { title, content } => {
                assert!(title.contains("notes"));
                assert!(!content.is_empty());
            }
            _ => panic!("Expected Preview for read-only verb, got {:?}", resp),
        }
    }

    #[test]
    fn submit_command_pending_confirmation_returns_confirmation() {
        let mut core = Core::new(make_storage());
        core.router_mut().register(calendar_metadata());

        {
            let mut ws = core.workspace_mut();
            ws.confirmation_pending = Some(ConfirmationPending {
                confirmation_id: "confirm-pending".to_string(),
                tool_id: "dangerous_op".to_string(),
                args: serde_json::json!({}),
                requested_at: 500,
            });
        }

        let resp = core.submit_command("schedule a meeting").unwrap();
        match resp {
            CoreResponse::Confirmation {
                confirmation_id,
                prompt,
                ..
            } => {
                assert_eq!(confirmation_id, "confirm-pending");
                assert!(prompt.contains("dangerous_op"));
            }
            _ => panic!("Expected Confirmation when pending, got {:?}", resp),
        }
    }

    #[test]
    fn follow_up_within_ttl_biases_router() {
        let mut core = Core::new(make_storage());
        core.router_mut().register(calendar_metadata());
        core.router_mut().register(notes_metadata());

        core.activate_follow_up(
            "create event at 2pm".to_string(),
            vec!["event-123".to_string()],
            "calendar".to_string(),
        );

        let resp = core.submit_command("make it 2:30").unwrap();
        match resp {
            CoreResponse::Artifact { content, .. } => {
                assert!(
                    content.contains("calendar"),
                    "Calendar should appear due to follow-up bias"
                );
            }
            _ => panic!("Expected Artifact response"),
        }
    }

    #[test]
    fn follow_up_after_ttl_expiry_triggers_error() {
        let ws = Workspace::new("test-session".to_string());
        let mut core = Core::with_state(ws, Router::new(), make_storage());
        core.router_mut().register(calendar_metadata());

        // Manually set follow-up with an already-expired TTL.
        {
            let mut ws = core.workspace_mut();
            ws.follow_up = Some(FollowUpContext {
                last_command: "create event".to_string(),
                last_result_entity_ids: vec!["event-123".to_string()],
                last_app_id: "calendar".to_string(),
                expires_at: 0, // Already expired (epoch).
                turn_count: 0,
                max_turns: FOLLOW_UP_MAX_TURNS,
            });
            ws.mode = WorkspaceMode::FollowUpActive;
        }

        let resp = core.submit_command("make it 2:30").unwrap();
        match resp {
            CoreResponse::Error { message } => {
                assert!(message.contains("expired"));
            }
            _ => panic!("Expected Error after TTL expiry"),
        }

        assert_eq!(core.workspace().mode, WorkspaceMode::Idle);
        assert!(core.workspace().follow_up.is_none());
    }

    #[test]
    fn follow_up_turn_limit_exhausts_context() {
        let mut core = Core::new(make_storage());
        core.router_mut().register(calendar_metadata());

        core.activate_follow_up(
            "create event".to_string(),
            vec!["event-123".to_string()],
            "calendar".to_string(),
        );

        // Consume turns up to the limit.
        for _ in 0..FOLLOW_UP_MAX_TURNS {
            let resp = core.submit_command("update it").unwrap();
            assert!(matches!(resp, CoreResponse::Artifact { .. }));
        }

        // Next command should trigger error since turns exhausted.
        let resp = core.submit_command("change the time").unwrap();
        match resp {
            CoreResponse::Error { message } => {
                assert!(message.contains("expired"));
            }
            CoreResponse::Artifact { .. } => {
                // Follow-up already expired; workspace mode should be Idle.
                assert_eq!(core.workspace().mode, WorkspaceMode::Idle);
            }
            _ => panic!("Expected Error or Artifact after exhaustion"),
        }
    }

    #[test]
    fn workspace_mode_transitions_correctly() {
        let mut core = Core::new(make_storage());

        assert_eq!(core.workspace().mode, WorkspaceMode::Idle);

        core.activate_follow_up(
            "test cmd".to_string(),
            vec!["id-1".to_string()],
            "test_app".to_string(),
        );
        assert_eq!(core.workspace().mode, WorkspaceMode::FollowUpActive);

        {
            let mut ws = core.workspace_mut();
            ws.expire_follow_up();
        }
        assert_eq!(core.workspace().mode, WorkspaceMode::Idle);
    }

    #[test]
    fn confirm_action_approve_returns_artifact() {
        let mut core = Core::new(make_storage());
        {
            let mut ws = core.workspace_mut();
            ws.confirmation_pending = Some(ConfirmationPending {
                confirmation_id: "confirm-abc".to_string(),
                tool_id: "delete_file".to_string(),
                args: serde_json::json!({"path": "/tmp/test"}),
                requested_at: 1000,
            });
            ws.mode = WorkspaceMode::AwaitingConfirmation;
        }

        let resp = core.confirm_action("confirm-abc", true).unwrap();
        match resp {
            CoreResponse::Artifact { content, .. } => {
                assert!(content.contains("confirmed"));
                assert!(content.contains("delete_file"));
            }
            _ => panic!("Expected Artifact for approved action"),
        }
        assert!(core.workspace().confirmation_pending.is_none());
    }

    #[test]
    fn confirm_action_deny_returns_artifact() {
        let mut core = Core::new(make_storage());
        {
            let mut ws = core.workspace_mut();
            ws.confirmation_pending = Some(ConfirmationPending {
                confirmation_id: "confirm-xyz".to_string(),
                tool_id: "rm_dir".to_string(),
                args: serde_json::json!({}),
                requested_at: 2000,
            });
            ws.mode = WorkspaceMode::AwaitingConfirmation;
        }

        let resp = core.confirm_action("confirm-xyz", false).unwrap();
        match resp {
            CoreResponse::Artifact { content, .. } => {
                assert!(content.contains("cancelled"));
                assert!(content.contains("rm_dir"));
            }
            _ => panic!("Expected Artifact for denied action"),
        }
    }

    #[test]
    fn confirm_action_wrong_id_returns_error() {
        let mut core = Core::new(make_storage());
        {
            let mut ws = core.workspace_mut();
            ws.confirmation_pending = Some(ConfirmationPending {
                confirmation_id: "confirm-abc".to_string(),
                tool_id: "test_tool".to_string(),
                args: serde_json::json!({}),
                requested_at: 1000,
            });
        }

        let resp = core.confirm_action("wrong-id", true).unwrap();
        match resp {
            CoreResponse::Error { message } => {
                assert!(message.contains("wrong-id"));
            }
            _ => panic!("Expected Error for wrong confirmation_id"),
        }
    }

    #[test]
    fn confirm_action_no_pending_returns_error() {
        let mut core = Core::new(make_storage());

        let resp = core.confirm_action("any-id", true).unwrap();
        match resp {
            CoreResponse::Error { message } => {
                assert!(message.contains("No action pending"));
            }
            _ => panic!("Expected Error when no confirmation pending"),
        }
    }

    #[test]
    fn get_workspace_snapshot_returns_workspace() {
        let core = Core::new(make_storage());
        let ws = core.get_workspace_snapshot().unwrap();
        assert_eq!(ws.mode, WorkspaceMode::Idle);
    }

    // --- Phase 5: Workspace snapshot save/load tests ---

    #[test]
    fn core_saves_snapshot_after_submit_command() {
        let mut core = Core::new(make_storage());
        core.router_mut().register(calendar_metadata());

        // Initially no snapshot.
        assert!(core.storage().snapshots().load().is_none());

        core.submit_command("schedule a meeting").unwrap();

        // Snapshot should be saved after command.
        let snapshot = core.storage().snapshots().load().unwrap();
        assert!(!snapshot.session_id.is_empty());
        // Data should deserialize back to a valid Workspace.
        let restored: Workspace =
            serde_json::from_value(snapshot.data).expect("snapshot data is valid Workspace");
        assert_eq!(restored.session_id, core.workspace().session_id);
    }

    #[test]
    fn core_saves_snapshot_after_confirm_action() {
        let mut core = Core::new(make_storage());
        {
            let mut ws = core.workspace_mut();
            ws.confirmation_pending = Some(ConfirmationPending {
                confirmation_id: "snap-confirm".to_string(),
                tool_id: "test_tool".to_string(),
                args: serde_json::json!({}),
                requested_at: 1000,
            });
            ws.mode = WorkspaceMode::AwaitingConfirmation;
        }

        core.confirm_action("snap-confirm", true).unwrap();

        let snapshot = core.storage().snapshots().load().unwrap();
        let restored: Workspace =
            serde_json::from_value(snapshot.data).expect("valid Workspace");
        // Confirmation should be cleared in the snapshot.
        assert!(restored.confirmation_pending.is_none());
        assert_eq!(restored.mode, WorkspaceMode::Idle);
    }

    #[test]
    fn core_restores_workspace_from_snapshot_on_new() {
        // Pre-seed storage with a snapshot containing a specific session_id.
        let mut storage = MemoryStorage::new();
        let ws = Workspace::new("restored-session".to_string());
        let data = serde_json::to_value(&ws).unwrap();
        storage.snapshots_mut().save(crate::storage::types::WorkspaceSnapshot {
            session_id: "restored-session".to_string(),
            captured_at: SystemTime::now(),
            data,
        });

        let core = Core::new(Box::new(storage));
        assert_eq!(core.workspace().session_id, "restored-session");
    }

    #[test]
    fn core_creates_fresh_workspace_when_no_snapshot() {
        let core = Core::new(make_storage());
        // Should have a valid UUID session_id, not empty.
        assert!(!core.workspace().session_id.is_empty());
        assert_eq!(core.workspace().mode, WorkspaceMode::Idle);
    }

    #[test]
    fn core_handles_corrupt_snapshot_gracefully() {
        // Pre-seed storage with invalid workspace data.
        let mut storage = MemoryStorage::new();
        storage.snapshots_mut().save(crate::storage::types::WorkspaceSnapshot {
            session_id: "bad-session".to_string(),
            captured_at: SystemTime::now(),
            data: serde_json::json!("not a workspace object"),
        });

        // Should fall back to a fresh workspace rather than panicking.
        let core = Core::new(Box::new(storage));
        assert_ne!(core.workspace().session_id, "bad-session");
        assert_eq!(core.workspace().mode, WorkspaceMode::Idle);

        // Should have logged a warning event.
        let events = core.storage().event_log().tail(10);
        assert_eq!(events.len(), 1);
        assert!(events[0].summary.contains("Error"));
    }

    #[test]
    fn core_clears_confirmation_pending_on_restore() {
        let mut ws = Workspace::new("confirm-session".to_string());
        ws.confirmation_pending = Some(ConfirmationPending {
            confirmation_id: "stale-confirm".to_string(),
            tool_id: "dangerous_op".to_string(),
            args: serde_json::json!({}),
            requested_at: 1000,
        });
        ws.mode = WorkspaceMode::AwaitingConfirmation;

        let mut storage = MemoryStorage::new();
        storage.snapshots_mut().save(crate::storage::types::WorkspaceSnapshot {
            session_id: "confirm-session".to_string(),
            captured_at: SystemTime::now(),
            data: serde_json::to_value(&ws).unwrap(),
        });

        let core = Core::new(Box::new(storage));
        assert_eq!(core.workspace().session_id, "confirm-session");
        // Ephemeral confirmation must not survive restart.
        assert!(core.workspace().confirmation_pending.is_none());
        assert_eq!(core.workspace().mode, WorkspaceMode::Idle);
    }

    #[test]
    fn core_clears_expired_follow_up_on_restore() {
        let mut ws = Workspace::new("followup-session".to_string());
        ws.follow_up = Some(FollowUpContext {
            last_command: "old command".to_string(),
            last_result_entity_ids: vec!["id-1".to_string()],
            last_app_id: "calendar".to_string(),
            expires_at: 0, // Already expired (epoch).
            turn_count: 0,
            max_turns: FOLLOW_UP_MAX_TURNS,
        });
        ws.mode = WorkspaceMode::FollowUpActive;

        let mut storage = MemoryStorage::new();
        storage.snapshots_mut().save(crate::storage::types::WorkspaceSnapshot {
            session_id: "followup-session".to_string(),
            captured_at: SystemTime::now(),
            data: serde_json::to_value(&ws).unwrap(),
        });

        let core = Core::new(Box::new(storage));
        assert_eq!(core.workspace().session_id, "followup-session");
        // Expired follow-up must be cleared on restore.
        assert!(core.workspace().follow_up.is_none());
        assert_eq!(core.workspace().mode, WorkspaceMode::Idle);
    }

    #[test]
    fn core_preserves_valid_follow_up_on_restore() {
        let future_ts = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 600; // 10 minutes from now.

        let mut ws = Workspace::new("valid-followup".to_string());
        ws.follow_up = Some(FollowUpContext {
            last_command: "create event".to_string(),
            last_result_entity_ids: vec!["evt-1".to_string()],
            last_app_id: "calendar".to_string(),
            expires_at: future_ts,
            turn_count: 0,
            max_turns: FOLLOW_UP_MAX_TURNS,
        });
        ws.mode = WorkspaceMode::FollowUpActive;

        let mut storage = MemoryStorage::new();
        storage.snapshots_mut().save(crate::storage::types::WorkspaceSnapshot {
            session_id: "valid-followup".to_string(),
            captured_at: SystemTime::now(),
            data: serde_json::to_value(&ws).unwrap(),
        });

        let core = Core::new(Box::new(storage));
        // Valid follow-up should survive restore.
        let workspace = core.workspace();
        assert!(workspace.follow_up.is_some());
        assert_eq!(workspace.mode, WorkspaceMode::FollowUpActive);
        let ctx = workspace.follow_up.as_ref().unwrap();
        assert_eq!(ctx.last_app_id, "calendar");
    }

    #[test]
    fn snapshot_updates_after_each_command() {
        let mut core = Core::new(make_storage());
        core.router_mut().register(calendar_metadata());

        core.submit_command("schedule a meeting").unwrap();
        let snap1 = core.storage().snapshots().load().unwrap();

        core.submit_command("create an event").unwrap();
        let snap2 = core.storage().snapshots().load().unwrap();

        // Snapshot is overwritten each time (single-slot semantics).
        assert_eq!(snap1.session_id, snap2.session_id);
        // captured_at may differ but data reflects latest state.
        assert!(snap2.captured_at >= snap1.captured_at);
    }

    #[test]
    fn get_recent_actions_empty_when_no_events() {
        let core = Core::new(make_storage());
        let actions = core.get_recent_actions(10).unwrap();
        assert!(actions.is_empty());
    }

    #[test]
    fn get_recent_actions_returns_safe_summaries() {
        let mut core = Core::new(make_storage());
        core.router_mut().register(calendar_metadata());

        core.submit_command("schedule a meeting").unwrap();
        core.submit_command("create an event").unwrap();

        let actions = core.get_recent_actions(10).unwrap();
        assert_eq!(actions.len(), 2);
        // Summaries include char count but never raw user text.
        assert_eq!(actions[0].description, "Command (18 chars)");
        assert_eq!(actions[1].description, "Command (15 chars)");
        // IDs are valid UUIDs.
        assert!(!actions[0].id.is_empty());
        assert_ne!(actions[0].id, actions[1].id);
    }

    #[test]
    fn get_recent_actions_respects_limit() {
        let mut core = Core::new(make_storage());
        core.router_mut().register(calendar_metadata());

        core.submit_command("first").unwrap();
        core.submit_command("second").unwrap();
        core.submit_command("third").unwrap();

        let actions = core.get_recent_actions(2).unwrap();
        assert_eq!(actions.len(), 2);
        // tail returns the last N; summaries distinguish by char count.
        assert_eq!(actions[0].description, "Command (6 chars)");
        assert_eq!(actions[1].description, "Command (5 chars)");
    }

    // --- Serde serialization tests (required by Core-12 test checklist) ---

    #[test]
    fn core_response_artifact_serializable() {
        let resp = CoreResponse::Artifact {
            content: "Routed to calendar".to_string(),
            actions: vec![ArtifactAction {
                id: "action-1".to_string(),
                label: "Confirm".to_string(),
            }],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Artifact"));
        assert!(json.contains("calendar"));
    }

    #[test]
    fn core_response_preview_serializable() {
        let resp = CoreResponse::Preview {
            title: "Last Note".to_string(),
            content: "Buy groceries".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Preview"));
        assert!(json.contains("Last Note"));
    }

    #[test]
    fn core_response_confirmation_serializable() {
        let resp = CoreResponse::Confirmation {
            confirmation_id: "confirm-12345".to_string(),
            prompt: "Delete this file?".to_string(),
            description: "This will permanently remove /tmp/test.txt".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Confirmation"));
        assert!(json.contains("confirm-12345"));
    }

    #[test]
    fn core_response_error_serializable() {
        let resp = CoreResponse::Error {
            message: "Something went wrong".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Error"));
        assert!(json.contains("Something went wrong"));
    }

    #[test]
    fn stub_command_produces_each_variant() {
        let mut core = Core::new(make_storage());
        core.router_mut().register(calendar_metadata());
        core.router_mut().register(notes_metadata());

        // Artifact: normal routing (non-preview verb).
        let resp = core.submit_command("schedule a meeting").unwrap();
        assert!(matches!(resp, CoreResponse::Artifact { .. }));

        // Preview: read-only verb triggers Preview when routing matches.
        let resp = core.submit_command("show last note").unwrap();
        assert!(
            matches!(resp, CoreResponse::Preview { .. }),
            "Expected Preview for read-only verb, got {:?}",
            resp
        );

        // Error: expired follow-up.
        {
            let mut ws = core.workspace_mut();
            ws.follow_up = Some(FollowUpContext {
                last_command: "test".to_string(),
                last_result_entity_ids: vec![],
                last_app_id: "calendar".to_string(),
                expires_at: 0,
                turn_count: 0,
                max_turns: FOLLOW_UP_MAX_TURNS,
            });
            ws.mode = WorkspaceMode::FollowUpActive;
        }
        let resp = core.submit_command("update").unwrap();
        assert!(matches!(resp, CoreResponse::Error { .. }));

        // Confirmation: pending confirmation blocks new commands.
        {
            let mut ws = core.workspace_mut();
            ws.confirmation_pending = Some(ConfirmationPending {
                confirmation_id: "c-1".to_string(),
                tool_id: "risky_tool".to_string(),
                args: serde_json::json!({}),
                requested_at: 100,
            });
        }
        let resp = core.submit_command("do something").unwrap();
        assert!(
            matches!(resp, CoreResponse::Confirmation { .. }),
            "Expected Confirmation when pending, got {:?}",
            resp
        );
    }
}
