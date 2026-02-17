use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Notify};

pub struct OAuthManager {
    flows: Mutex<HashMap<String, PendingFlow>>,
    timeout: Duration,
}

struct PendingFlow {
    authorization_code: Option<String>,
    created_at: Instant,
    notify: Arc<Notify>,
}

impl OAuthManager {
    pub fn new(timeout: Duration) -> Self {
        Self {
            flows: Mutex::new(HashMap::new()),
            timeout,
        }
    }

    pub async fn register_flow(&self, state: &str) -> Result<(), String> {
        let mut flows = self.flows.lock().await;
        if flows.contains_key(state) {
            return Err("duplicate state parameter".to_string());
        }
        flows.insert(
            state.to_string(),
            PendingFlow {
                authorization_code: None,
                created_at: Instant::now(),
                notify: Arc::new(Notify::new()),
            },
        );
        Ok(())
    }

    pub async fn complete_flow(&self, state: &str, code: &str) -> Result<(), String> {
        let mut flows = self.flows.lock().await;
        let flow = flows
            .get_mut(state)
            .ok_or_else(|| "unknown state parameter".to_string())?;
        flow.authorization_code = Some(code.to_string());
        flow.notify.notify_waiters();
        Ok(())
    }

    /// Long-poll for the authorization code. Returns the code if completed within
    /// `wait_secs`, or `None` on timeout. Consumes the flow on success.
    pub async fn poll_flow(&self, state: &str, wait_secs: u64) -> Result<Option<String>, String> {
        let notify = {
            let flows = self.flows.lock().await;
            let flow = flows
                .get(state)
                .ok_or_else(|| "unknown state parameter".to_string())?;
            if let Some(code) = &flow.authorization_code {
                let code = code.clone();
                drop(flows);
                let mut flows = self.flows.lock().await;
                flows.remove(state);
                return Ok(Some(code));
            }
            flow.notify.clone()
        };

        let wait = Duration::from_secs(wait_secs);
        let result = tokio::time::timeout(wait, notify.notified()).await;

        if result.is_ok() {
            let mut flows = self.flows.lock().await;
            if let Some(flow) = flows.remove(state) {
                return Ok(flow.authorization_code);
            }
        }

        Ok(None)
    }

    pub async fn cleanup(&self) {
        let mut flows = self.flows.lock().await;
        flows.retain(|_, flow| flow.created_at.elapsed() < self.timeout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_and_complete_flow() {
        let mgr = OAuthManager::new(Duration::from_secs(300));
        mgr.register_flow("abc").await.unwrap();
        mgr.complete_flow("abc", "code123").await.unwrap();
        let result = mgr.poll_flow("abc", 1).await.unwrap();
        assert_eq!(result, Some("code123".to_string()));
    }

    #[tokio::test]
    async fn duplicate_state_rejected() {
        let mgr = OAuthManager::new(Duration::from_secs(300));
        mgr.register_flow("dup").await.unwrap();
        let err = mgr.register_flow("dup").await.unwrap_err();
        assert_eq!(err, "duplicate state parameter");
    }

    #[tokio::test]
    async fn poll_timeout_returns_none() {
        let mgr = OAuthManager::new(Duration::from_secs(300));
        mgr.register_flow("wait").await.unwrap();
        let result = mgr.poll_flow("wait", 1).await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn cleanup_removes_expired() {
        let mgr = OAuthManager::new(Duration::from_millis(1));
        mgr.register_flow("old").await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        mgr.cleanup().await;
        let err = mgr.poll_flow("old", 0).await.unwrap_err();
        assert_eq!(err, "unknown state parameter");
    }

    #[tokio::test]
    async fn poll_consumes_flow() {
        let mgr = OAuthManager::new(Duration::from_secs(300));
        mgr.register_flow("once").await.unwrap();
        mgr.complete_flow("once", "code").await.unwrap();
        let first = mgr.poll_flow("once", 1).await.unwrap();
        assert_eq!(first, Some("code".to_string()));
        let second = mgr.poll_flow("once", 0).await;
        assert!(second.is_err());
    }
}
