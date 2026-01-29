use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};
use crate::workspace::{WorkspaceConfig, WorkspaceInstance};

const SESSION_STORE_VERSION: &str = "1.0.0";
const DEFAULT_CONTEXT_LIMIT: usize = 50;
static SESSION_STORE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub seq: u64,
    pub timestamp: String,
    pub role: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: String,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub messages: Vec<SessionMessage>,
    pub next_seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStore {
    pub version: String,
    pub sessions: Vec<SessionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub workspace_id: String,
    pub session_id: String,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub messages: Vec<SessionMessage>,
}

pub fn record_user_message(
    workspace: &WorkspaceInstance,
    text: &str,
) -> CoreResult<SessionContext> {
    let _guard = lock_session_store()?;
    let mut store = load_or_create_session_store(&workspace.sessions_path())?;
    let now = now_secs();
    let duration = workspace.config.preferences.session.duration_seconds;

    let session = get_or_start_session(&mut store, now, duration);
    let message = SessionMessage {
        seq: session.next_seq,
        timestamp: now_rfc3339(),
        role: "user".to_string(),
        text: text.to_string(),
    };
    session.next_seq = session.next_seq.saturating_add(1);
    session.messages.push(message);

    let session_snapshot = session.clone();
    save_session_store(&workspace.sessions_path(), &store)?;
    build_session_context(&workspace.config, &session_snapshot, None)
}

pub fn get_session_context(
    workspace: &WorkspaceInstance,
    session_id: Option<&str>,
    limit: Option<usize>,
) -> CoreResult<SessionContext> {
    let _guard = lock_session_store()?;
    let store = load_or_create_session_store(&workspace.sessions_path())?;
    let selected = match session_id {
        Some(id) => store
            .sessions
            .iter()
            .find(|session| session.session_id == id)
            .ok_or_else(|| CoreError::InvalidInput("session not found".to_string()))?,
        None => store
            .sessions
            .last()
            .ok_or_else(|| CoreError::InvalidInput("no sessions found".to_string()))?,
    };

    build_session_context(&workspace.config, selected, limit)
}

pub fn open_application(
    workspace: &WorkspaceInstance,
    app_id: &str,
) -> CoreResult<SessionContext> {
    let session_id = ensure_active_session(workspace)?;
    let now = now_secs();
    workspace.open_application(&session_id, app_id, now);
    get_session_context(workspace, Some(&session_id), None)
}

pub fn close_application(
    workspace: &WorkspaceInstance,
    app_id: &str,
) -> CoreResult<SessionContext> {
    let session_id = ensure_active_session(workspace)?;
    workspace.close_application(&session_id, app_id);
    get_session_context(workspace, Some(&session_id), None)
}

fn build_session_context(
    config: &WorkspaceConfig,
    session: &SessionRecord,
    limit: Option<usize>,
) -> CoreResult<SessionContext> {
    let cap = limit.unwrap_or(DEFAULT_CONTEXT_LIMIT);
    let messages = if session.messages.len() > cap {
        session.messages[session.messages.len() - cap..].to_vec()
    } else {
        session.messages.clone()
    };

    Ok(SessionContext {
        workspace_id: config.workspace_id.clone(),
        session_id: session.session_id.clone(),
        started_at: session.started_at,
        ended_at: session.ended_at,
        messages,
    })
}

fn load_or_create_session_store(path: &Path) -> CoreResult<SessionStore> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            CoreError::Internal(format!(
                "failed to create workspace directory {}: {error}",
                parent.display()
            ))
        })?;
    }

    if !path.exists() {
        let store = SessionStore {
            version: SESSION_STORE_VERSION.to_string(),
            sessions: Vec::new(),
        };
        save_session_store(path, &store)?;
        return Ok(store);
    }

    let data = std::fs::read_to_string(path).map_err(|error| {
        CoreError::Internal(format!(
            "failed to read session store {}: {error}",
            path.display()
        ))
    })?;
    let store: SessionStore = serde_json::from_str(&data).map_err(|error| {
        CoreError::Internal(format!(
            "failed to parse session store {}: {error}",
            path.display()
        ))
    })?;

    Ok(store)
}

fn ensure_active_session(workspace: &WorkspaceInstance) -> CoreResult<String> {
    let _guard = lock_session_store()?;
    let mut store = load_or_create_session_store(&workspace.sessions_path())?;
    let now = now_secs();
    let duration = workspace.config.preferences.session.duration_seconds;
    let session = get_or_start_session(&mut store, now, duration);
    let session_id = session.session_id.clone();
    save_session_store(&workspace.sessions_path(), &store)?;
    Ok(session_id)
}

fn lock_session_store() -> CoreResult<MutexGuard<'static, ()>> {
    SESSION_STORE_LOCK
        .lock()
        .map_err(|_| CoreError::Internal("session store lock poisoned".to_string()))
}

fn save_session_store(path: &Path, store: &SessionStore) -> CoreResult<()> {
    let data = serde_json::to_string_pretty(store).map_err(|error| {
        CoreError::Internal(format!(
            "failed to serialize session store {}: {error}",
            path.display()
        ))
    })?;
    std::fs::write(path, data).map_err(|error| {
        CoreError::Internal(format!(
            "failed to write session store {}: {error}",
            path.display()
        ))
    })?;
    Ok(())
}

fn get_or_start_session<'a>(
    store: &'a mut SessionStore,
    now: u64,
    duration_seconds: u64,
) -> &'a mut SessionRecord {
    let expired = store.sessions.last().map(|session| {
        now.saturating_sub(session.started_at) >= duration_seconds
    });

    if store.sessions.is_empty() || expired.unwrap_or(true) {
        if let Some(last) = store.sessions.last_mut() {
            last.ended_at = Some(now);
        }
        store.sessions.push(SessionRecord {
            session_id: Uuid::new_v4().to_string(),
            started_at: now,
            ended_at: None,
            messages: Vec::new(),
            next_seq: 1,
        });
    }

    store.sessions.last_mut().expect("session exists")
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn now_rfc3339() -> String {
    let now = std::time::SystemTime::now();
    let datetime: chrono::DateTime<chrono::Utc> = now.into();
    datetime.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::WorkspaceInstance;
    use tempfile::tempdir;

    fn force_expire_session(workspace: &WorkspaceInstance) {
        let path = workspace.sessions_path();
        let mut store = load_or_create_session_store(&path).expect("load store");
        if let Some(last) = store.sessions.last_mut() {
            last.started_at = 0;
            last.ended_at = Some(0);
        }
        save_session_store(&path, &store).expect("save store");
    }

    #[test]
    fn record_user_message_creates_session_store() {
        let dir = tempdir().expect("tempdir");
        let workspace = WorkspaceInstance::load(dir.path()).expect("workspace");
        let ctx = record_user_message(&workspace, "hello").expect("record");
        assert_eq!(ctx.messages.len(), 1);
        assert_eq!(ctx.messages[0].text, "hello");
    }

    #[test]
    fn get_session_context_defaults_to_latest() {
        let dir = tempdir().expect("tempdir");
        let workspace = WorkspaceInstance::load(dir.path()).expect("workspace");
        record_user_message(&workspace, "first").expect("record");
        let ctx = get_session_context(&workspace, None, None).expect("context");
        assert_eq!(ctx.messages.len(), 1);
        assert_eq!(ctx.messages[0].text, "first");
    }

    #[test]
    fn open_application_evicts_oldest() {
        let dir = tempdir().expect("tempdir");
        let workspace = WorkspaceInstance::load(dir.path()).expect("workspace");
        assert_eq!(
            workspace.config.preferences.application_cache.max_applications,
            8
        );

        for idx in 0..9 {
            let app_id = format!("app-{}", idx);
            open_application(&workspace, &app_id).expect("open");
        }

        let ctx = get_session_context(&workspace, None, None).expect("context");
        assert_eq!(ctx.messages.len(), 0);
    }

    #[test]
    fn application_cache_resets_on_session_rollover() {
        let dir = tempdir().expect("tempdir");
        let mut workspace = WorkspaceInstance::load(dir.path()).expect("workspace");
        workspace.config.preferences.session.duration_seconds = 0;

        open_application(&workspace, "old-app").expect("open");
        force_expire_session(&workspace);

        open_application(&workspace, "new-app").expect("open");
        let ctx = get_session_context(&workspace, None, None).expect("context");
        assert_eq!(ctx.messages.len(), 0);
    }
}
