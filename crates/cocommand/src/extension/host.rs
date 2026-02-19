use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin};
use tokio::sync::{oneshot, Mutex};
use tokio::time::{timeout, Duration};

use crate::error::{CoreError, CoreResult};

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct RpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RpcResponse {
    Success {
        #[serde(rename = "jsonrpc")]
        _jsonrpc: String,
        id: u64,
        result: serde_json::Value,
    },
    Error {
        #[serde(rename = "jsonrpc")]
        _jsonrpc: String,
        id: u64,
        error: RpcError,
    },
}

#[derive(Debug, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(Debug, Serialize)]
struct InitializeParams<'a> {
    extension_dir: &'a str,
    extension_id: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct InitializeResult {
    pub tools: Vec<String>,
}

#[derive(Debug, Serialize)]
struct InvokeToolParams<'a> {
    tool_id: &'a str,
    args: serde_json::Value,
}

pub struct ExtensionHost {
    stdin: Arc<Mutex<ChildStdin>>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<RpcResponse>>>>,
    next_id: AtomicU64,
    _child: Child,
}

impl ExtensionHost {
    pub async fn start(extension_host_path: &Path) -> CoreResult<Self> {
        let deno_path = resolve_deno_binary()?;
        let mut cmd = tokio::process::Command::new(deno_path);
        cmd.arg("run")
            .arg("--no-check")
            .arg("--allow-read")
            .arg("--allow-env")
            .arg("--quiet")
            .arg(extension_host_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|error| {
            CoreError::Internal(format!("failed to spawn extension host: {error}"))
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| CoreError::Internal("extension host stdin unavailable".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CoreError::Internal("extension host stdout unavailable".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| CoreError::Internal("extension host stderr unavailable".to_string()))?;

        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<RpcResponse>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_clone = pending.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                let response: Result<RpcResponse, _> = serde_json::from_str(&line);
                match response {
                    Ok(response) => {
                        let id = match &response {
                            RpcResponse::Success { id, .. } => *id,
                            RpcResponse::Error { id, .. } => *id,
                        };
                        if let Some(tx) = pending_clone.lock().await.remove(&id) {
                            let _ = tx.send(response);
                        }
                    }
                    Err(error) => {
                        tracing::warn!("extension-host stdout parse error: {} line={}", error, line);
                    }
                }
            }
        });

        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                tracing::warn!("extension-host stderr: {}", line);
            }
        });

        Ok(Self {
            stdin: Arc::new(Mutex::new(stdin)),
            pending,
            next_id: AtomicU64::new(1),
            _child: child,
        })
    }

    pub async fn initialize(
        &self,
        extension_dir: &Path,
        extension_id: &str,
    ) -> CoreResult<InitializeResult> {
        let params = InitializeParams {
            extension_dir: extension_dir
                .to_str()
                .ok_or_else(|| CoreError::Internal("invalid extension path".to_string()))?,
            extension_id,
        };
        let result = self
            .send_request(
                "initialize",
                serde_json::to_value(params).map_err(|error| {
                    CoreError::Internal(format!("failed to serialize init params: {error}"))
                })?,
            )
            .await?;
        serde_json::from_value(result)
            .map_err(|error| CoreError::Internal(format!("failed to parse init response: {error}")))
    }

    pub async fn invoke_tool(
        &self,
        tool_id: &str,
        args: serde_json::Value,
    ) -> CoreResult<serde_json::Value> {
        let params = InvokeToolParams { tool_id, args };
        let result = self
            .send_request(
                "invoke_tool",
                serde_json::to_value(params).map_err(|error| {
                    CoreError::Internal(format!("failed to serialize invoke params: {error}"))
                })?,
            )
            .await?;
        Ok(result)
    }

    async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> CoreResult<serde_json::Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = RpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };
        let payload = serde_json::to_string(&request).map_err(|error| {
            CoreError::Internal(format!("failed to serialize rpc request: {error}"))
        })?;
        let _ = method;

        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);

        let mut stdin = self.stdin.lock().await;
        stdin.write_all(payload.as_bytes()).await.map_err(|error| {
            CoreError::Internal(format!("failed to write rpc request: {error}"))
        })?;
        stdin.write_all(b"\n").await.map_err(|error| {
            CoreError::Internal(format!("failed to write rpc request: {error}"))
        })?;
        stdin.flush().await.map_err(|error| {
            CoreError::Internal(format!("failed to flush rpc request: {error}"))
        })?;

        let response = timeout(Duration::from_secs(10), rx)
            .await
            .map_err(|_| CoreError::Internal(format!("rpc timeout for method {}", method)))?
            .map_err(|_| CoreError::Internal("rpc response dropped".to_string()))?;
        match response {
            RpcResponse::Success { result, .. } => Ok(result),
            RpcResponse::Error { error, .. } => Err(CoreError::Internal(format!(
                "extension host error {}: {}",
                error.code, error.message
            ))),
        }
    }
}

pub fn extension_host_entrypoint() -> CoreResult<PathBuf> {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_dir
        .parent()
        .and_then(|path| path.parent())
        .ok_or_else(|| CoreError::Internal("failed to resolve repo root".to_string()))?;
    Ok(repo_root.join("apps/extension-host/main.ts"))
}

fn resolve_deno_binary() -> CoreResult<PathBuf> {
    if let Ok(path) = std::env::var("COCOMMAND_DENO_PATH") {
        return Ok(PathBuf::from(path));
    }

    if let Ok(exe) = std::env::current_exe() {
        // macOS app bundle: Cocommand.app/Contents/MacOS/<binary>
        if let Some(contents_dir) = exe.parent().and_then(|p| p.parent()) {
            if contents_dir.ends_with("Contents") {
                let bundled = contents_dir.join("Resources").join("deno");
                if bundled.exists() {
                    return Ok(bundled);
                }
            }
        }

        // dev fallback: apps/desktop/src-tauri/resources/deno
        if let Some(repo_root) = exe.parent().and_then(|p| p.parent()) {
            let dev_bundled = repo_root
                .join("apps")
                .join("desktop")
                .join("src-tauri")
                .join("resources")
                .join("deno");
            if dev_bundled.exists() {
                return Ok(dev_bundled);
            }
        }
    }

    Ok(PathBuf::from("deno"))
}
