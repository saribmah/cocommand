//! Extension lifecycle management: install, load, unload, tool catalog sync.
//!
//! The `ExtensionManager` is responsible for loading extension manifests,
//! spawning Deno host processes, registering extension tools in the tool
//! registry, and syncing routing metadata with the router.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;

use crate::error::{CoreError, CoreResult};
use crate::routing::{Router, RoutingMetadata};
use crate::tools::registry::ToolRegistry;
use crate::tools::schema::ToolDefinition;

use super::manifest::ExtensionManifest;
use super::rpc::{InvokeToolResult, RpcRequest, RpcResponse};

/// Default timeout for tool invocations (5 seconds).
const DEFAULT_TIMEOUT_MS: u64 = 5000;

/// Handle to a running extension host process.
struct ExtensionHost {
    process: Child,
    stdin: Arc<Mutex<Box<dyn Write + Send>>>,
    stdout: Arc<Mutex<BufReader<Box<dyn Read + Send>>>>,
    next_id: Arc<AtomicU64>,
}

/// A loaded extension with its manifest and host process.
struct LoadedExtension {
    manifest: ExtensionManifest,
    _host: ExtensionHost,
    #[allow(dead_code)]
    extension_dir: PathBuf,
}

/// Manages the lifecycle of extensions: install, load, unload.
pub struct ExtensionManager {
    extensions: HashMap<String, LoadedExtension>,
    /// Timeout for tool invocations in milliseconds.
    pub timeout_ms: u64,
    /// Path to the extension host entrypoint.
    host_entrypoint: PathBuf,
}

impl ExtensionManager {
    /// Create a new extension manager.
    ///
    /// `host_entrypoint` is the path to the Deno extension host `main.ts`.
    pub fn new(host_entrypoint: PathBuf) -> Self {
        Self {
            extensions: HashMap::new(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            host_entrypoint,
        }
    }

    /// Install and load an extension from a directory.
    ///
    /// Reads the manifest, spawns the Deno host, initializes it, and registers
    /// tools and routing metadata.
    pub fn load_extension(
        &mut self,
        extension_dir: &Path,
        registry: &mut ToolRegistry,
        router: &mut Router,
    ) -> CoreResult<String> {
        let manifest = self.read_manifest(extension_dir)?;
        let ext_id = manifest.id.clone();

        if self.extensions.contains_key(&ext_id) {
            return Err(CoreError::InvalidInput(format!(
                "extension already loaded: {ext_id}"
            )));
        }

        let host = self.spawn_host(extension_dir, &manifest)?;

        // Register tools in the tool registry
        self.register_tools(&manifest, &host, registry)?;

        // Register routing metadata
        self.register_routing(&manifest, router);

        self.extensions.insert(
            ext_id.clone(),
            LoadedExtension {
                manifest,
                _host: host,
                extension_dir: extension_dir.to_path_buf(),
            },
        );

        Ok(ext_id)
    }

    /// Unload an extension: kill host process, remove tools and routing.
    pub fn unload_extension(
        &mut self,
        extension_id: &str,
        registry: &mut ToolRegistry,
    ) -> CoreResult<()> {
        let ext = self.extensions.remove(extension_id).ok_or_else(|| {
            CoreError::InvalidInput(format!("extension not loaded: {extension_id}"))
        })?;

        // Remove instance tools for this extension
        registry.remove_instance_tools(extension_id);

        // Kill the host process
        drop(ext);

        Ok(())
    }

    /// Get the manifest for a loaded extension.
    pub fn get_manifest(&self, extension_id: &str) -> Option<&ExtensionManifest> {
        self.extensions.get(extension_id).map(|e| &e.manifest)
    }

    /// List loaded extension IDs.
    pub fn loaded_extensions(&self) -> Vec<&str> {
        self.extensions.keys().map(|s| s.as_str()).collect()
    }

    /// Read and parse the extension manifest from the given directory.
    fn read_manifest(&self, extension_dir: &Path) -> CoreResult<ExtensionManifest> {
        let manifest_path = extension_dir.join("manifest.json");
        let content = std::fs::read_to_string(&manifest_path).map_err(|e| {
            CoreError::Internal(format!(
                "failed to read manifest at {}: {e}",
                manifest_path.display()
            ))
        })?;
        serde_json::from_str(&content).map_err(|e| {
            CoreError::Internal(format!("failed to parse manifest: {e}"))
        })
    }

    /// Spawn the Deno extension host process and initialize it.
    fn spawn_host(
        &self,
        extension_dir: &Path,
        manifest: &ExtensionManifest,
    ) -> CoreResult<ExtensionHost> {
        let mut child = Command::new("deno")
            .arg("run")
            .arg("--allow-read")
            .arg("--allow-env")
            .arg(self.host_entrypoint.to_string_lossy().as_ref())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| CoreError::Internal(format!("failed to spawn Deno host: {e}")))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| CoreError::Internal("failed to capture host stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CoreError::Internal("failed to capture host stdout".to_string()))?;

        let stdin: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(stdin)));
        let stdout: Arc<Mutex<BufReader<Box<dyn Read + Send>>>> =
            Arc::new(Mutex::new(BufReader::new(Box::new(stdout))));
        let next_id = Arc::new(AtomicU64::new(1));

        let host = ExtensionHost {
            process: child,
            stdin: stdin.clone(),
            stdout: stdout.clone(),
            next_id: next_id.clone(),
        };

        // Send initialize request
        let init_params = json!({
            "extension_dir": extension_dir.to_string_lossy(),
            "extension_id": manifest.id,
        });
        let response = rpc_call(&stdin, &stdout, &next_id, "initialize", Some(init_params), self.timeout_ms)?;

        if response.is_error() {
            let err = response.error.unwrap();
            return Err(CoreError::Internal(format!(
                "host initialization failed: {}",
                err.message
            )));
        }

        Ok(host)
    }

    /// Register extension tools in the tool registry.
    fn register_tools(
        &self,
        manifest: &ExtensionManifest,
        host: &ExtensionHost,
        registry: &mut ToolRegistry,
    ) -> CoreResult<()> {
        for tool_def in &manifest.tools {
            let tool_id = tool_def.id.clone();
            let stdin = host.stdin.clone();
            let stdout = host.stdout.clone();
            let next_id = host.next_id.clone();
            let timeout_ms = self.timeout_ms;

            let handler: crate::tools::schema::ToolHandler = Box::new(
                move |args: &serde_json::Value,
                      _ctx: &mut crate::tools::schema::ExecutionContext| {
                    let params = json!({
                        "tool_id": tool_id,
                        "args": args,
                    });
                    let response =
                        rpc_call(&stdin, &stdout, &next_id, "invoke_tool", Some(params), timeout_ms)?;

                    if let Some(err) = response.error {
                        return Err(CoreError::Internal(format!(
                            "tool invocation failed: {}",
                            err.message
                        )));
                    }

                    let result_value = response.result.unwrap_or(json!(null));
                    let invoke_result: InvokeToolResult =
                        serde_json::from_value(result_value).map_err(|e| {
                            CoreError::Internal(format!("invalid tool result: {e}"))
                        })?;

                    Ok(invoke_result.output)
                },
            );

            let definition = ToolDefinition {
                tool_id: tool_def.id.clone(),
                input_schema: tool_def.input_schema.clone(),
                output_schema: tool_def.output_schema.clone(),
                risk_level: tool_def.risk_level.clone(),
                is_kernel: false,
                handler,
            };

            registry.register_instance_tool(manifest.id.clone(), definition);
        }

        Ok(())
    }

    /// Register extension routing metadata with the router.
    fn register_routing(&self, manifest: &ExtensionManifest, router: &mut Router) {
        let metadata = RoutingMetadata {
            app_id: manifest.id.clone(),
            keywords: manifest.routing.keywords.clone(),
            examples: manifest.routing.examples.clone(),
            verbs: manifest.routing.verbs.clone(),
            objects: manifest.routing.objects.clone(),
        };
        router.register(metadata);
    }
}

/// Perform a synchronous JSON-RPC call over stdio.
fn rpc_call(
    stdin: &Arc<Mutex<Box<dyn Write + Send>>>,
    stdout: &Arc<Mutex<BufReader<Box<dyn Read + Send>>>>,
    next_id: &Arc<AtomicU64>,
    method: &str,
    params: Option<serde_json::Value>,
    timeout_ms: u64,
) -> CoreResult<RpcResponse> {
    let id = next_id.fetch_add(1, Ordering::SeqCst);
    let request = RpcRequest::new(id, method, params);
    let request_json = serde_json::to_string(&request).map_err(|e| {
        CoreError::Internal(format!("failed to serialize request: {e}"))
    })?;

    // Write request to stdin
    {
        let mut stdin_lock = stdin.lock().map_err(|e| {
            CoreError::Internal(format!("stdin lock poisoned: {e}"))
        })?;
        writeln!(stdin_lock, "{}", request_json).map_err(|e| {
            CoreError::Internal(format!("failed to write to host stdin: {e}"))
        })?;
        stdin_lock.flush().map_err(|e| {
            CoreError::Internal(format!("failed to flush host stdin: {e}"))
        })?;
    }

    // Read response from stdout with timeout
    let response_line = read_line_with_timeout(stdout, Duration::from_millis(timeout_ms))?;

    let response: RpcResponse = serde_json::from_str(&response_line).map_err(|e| {
        CoreError::Internal(format!("failed to parse host response: {e}"))
    })?;

    if response.id != id {
        return Err(CoreError::Internal(format!(
            "response ID mismatch: expected {id}, got {}",
            response.id
        )));
    }

    Ok(response)
}

/// Read a line from a buffered reader with a timeout.
///
/// Spawns a blocking reader thread and uses `recv_timeout` on a channel
/// to enforce the deadline. The reader thread is detached on timeout.
fn read_line_with_timeout(
    stdout: &Arc<Mutex<BufReader<Box<dyn Read + Send>>>>,
    timeout: Duration,
) -> CoreResult<String> {
    let stdout = Arc::clone(stdout);
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let read_result = (|| -> Result<String, String> {
            let mut reader = stdout.lock().map_err(|e| format!("stdout lock poisoned: {e}"))?;
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => Err("host process closed stdout (EOF)".to_string()),
                Ok(_) => Ok(line.trim().to_string()),
                Err(e) => Err(format!("failed to read from host stdout: {e}")),
            }
        })();
        let _ = tx.send(read_result);
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(line)) => Ok(line),
        Ok(Err(e)) => Err(CoreError::Internal(e)),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            Err(CoreError::Internal("timeout waiting for host response".to_string()))
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            Err(CoreError::Internal("reader thread terminated unexpectedly".to_string()))
        }
    }
}

impl Drop for ExtensionHost {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use std::io::Cursor;

    /// Create a mock host with pre-configured stdin/stdout for testing.
    fn mock_rpc_call(
        response_json: &str,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> CoreResult<RpcResponse> {
        // stdout: returns the response
        let stdout_data = format!("{}\n", response_json);
        let stdout_reader: Box<dyn Read + Send> = Box::new(Cursor::new(stdout_data.into_bytes()));
        let stdout = Arc::new(Mutex::new(BufReader::new(stdout_reader)));

        // stdin: capture writes (we use a Vec<u8> sink)
        let stdin_writer: Box<dyn Write + Send> = Box::new(Vec::<u8>::new());
        let stdin = Arc::new(Mutex::new(stdin_writer));

        let next_id = Arc::new(AtomicU64::new(1));

        rpc_call(&stdin, &stdout, &next_id, method, params, 5000)
    }

    #[test]
    fn rpc_call_success() {
        let response = serde_json::to_string(&RpcResponse::success(
            1,
            json!({"output": {"ticket_id": "T-1"}}),
        ))
        .unwrap();

        let result = mock_rpc_call(&response, "invoke_tool", Some(json!({"tool_id": "test"}))).unwrap();
        assert!(!result.is_error());
        assert_eq!(result.result.unwrap()["output"]["ticket_id"], "T-1");
    }

    #[test]
    fn rpc_call_error_response() {
        let response = serde_json::to_string(&RpcResponse::error(
            1,
            -32000,
            "tool failed",
        ))
        .unwrap();

        let result = mock_rpc_call(&response, "invoke_tool", None).unwrap();
        assert!(result.is_error());
        assert_eq!(result.error.unwrap().message, "tool failed");
    }

    #[test]
    fn rpc_call_eof_returns_error() {
        // Empty stdout simulates host closing
        let stdout_reader: Box<dyn Read + Send> = Box::new(Cursor::new(Vec::<u8>::new()));
        let stdout = Arc::new(Mutex::new(BufReader::new(stdout_reader)));

        let stdin_writer: Box<dyn Write + Send> = Box::new(Vec::<u8>::new());
        let stdin = Arc::new(Mutex::new(stdin_writer));

        let next_id = Arc::new(AtomicU64::new(1));

        let result = rpc_call(&stdin, &stdout, &next_id, "test", None, 5000);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("closed stdout"));
    }

    #[test]
    fn extension_manager_new() {
        let mgr = ExtensionManager::new(PathBuf::from("/path/to/host/main.ts"));
        assert_eq!(mgr.timeout_ms, DEFAULT_TIMEOUT_MS);
        assert!(mgr.loaded_extensions().is_empty());
    }

    #[test]
    fn read_manifest_missing_file() {
        let mgr = ExtensionManager::new(PathBuf::from("main.ts"));
        let result = mgr.read_manifest(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn read_manifest_valid() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = json!({
            "id": "test-ext",
            "name": "Test Extension",
            "description": "A test",
            "entrypoint": "main.ts",
            "routing": {
                "keywords": ["test"]
            },
            "tools": [{
                "id": "test.tool",
                "risk_level": "Safe",
                "input_schema": {"type": "object"},
                "output_schema": {}
            }]
        });
        std::fs::write(
            dir.path().join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let mgr = ExtensionManager::new(PathBuf::from("main.ts"));
        let result = mgr.read_manifest(dir.path()).unwrap();
        assert_eq!(result.id, "test-ext");
        assert_eq!(result.tools.len(), 1);
    }

    #[test]
    fn register_routing_adds_metadata() {
        let manifest = ExtensionManifest {
            id: "ext-1".to_string(),
            name: "Ext".to_string(),
            description: "d".to_string(),
            entrypoint: "main.ts".to_string(),
            routing: super::super::manifest::ExtensionRouting {
                keywords: vec!["ticket".to_string()],
                examples: vec!["create a ticket".to_string()],
                verbs: vec!["create".to_string()],
                objects: vec!["ticket".to_string()],
            },
            tools: vec![],
        };

        let mgr = ExtensionManager::new(PathBuf::from("main.ts"));
        let mut router = Router::new();
        mgr.register_routing(&manifest, &mut router);

        // Verify routing works
        let cmd = crate::command::ParsedCommand {
            raw_text: "create a ticket".to_string(),
            normalized_text: "create a ticket".to_string(),
            tags: vec![],
        };
        let result = router.route(&cmd);
        assert!(!result.candidates.is_empty());
        assert_eq!(result.candidates[0].app_id, "ext-1");
    }

    /// Integration test: spawn actual Deno host and invoke a tool end-to-end.
    ///
    /// Requires Deno to be installed. Skipped if `deno` is not on PATH.
    #[test]
    fn integration_spawn_host_and_invoke_tool() {
        // Check if Deno is available
        if Command::new("deno").arg("--version").output().is_err() {
            eprintln!("skipping integration test: deno not found");
            return;
        }

        // Resolve paths relative to the crate root
        let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let project_root = crate_root.parent().unwrap().parent().unwrap();
        let host_entrypoint = project_root.join("apps/extension-host/main.ts");
        let extension_dir = project_root.join("extensions/sample-my-app");

        // Verify files exist
        assert!(
            host_entrypoint.exists(),
            "host entrypoint not found: {}",
            host_entrypoint.display()
        );
        assert!(
            extension_dir.join("manifest.json").exists(),
            "sample extension manifest not found"
        );

        let mut mgr = ExtensionManager::new(host_entrypoint);
        let mut registry = ToolRegistry::new();
        let mut router = Router::new();

        // Load the sample extension
        let ext_id = mgr
            .load_extension(&extension_dir, &mut registry, &mut router)
            .expect("should load extension");
        assert_eq!(ext_id, "my-app");

        // Verify tools are registered
        let tool = registry.lookup("my-app", "my_app.create_ticket");
        assert!(tool.is_some(), "tool should be registered");

        // Verify routing metadata is registered
        let cmd = crate::command::ParsedCommand {
            raw_text: "create a ticket".to_string(),
            normalized_text: "create a ticket".to_string(),
            tags: vec![],
        };
        let routing_result = router.route(&cmd);
        assert!(
            routing_result.candidates.iter().any(|c| c.app_id == "my-app"),
            "extension should appear in routing candidates"
        );

        // Invoke the tool
        let tool = registry.lookup("my-app", "my_app.create_ticket").unwrap();
        let mut workspace = crate::workspace::Workspace::new("test".to_string());
        let mut storage: Box<dyn Storage> = Box::new(crate::storage::MemoryStorage::new());
        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let mut ctx = crate::tools::schema::ExecutionContext {
            workspace: &mut workspace,
            event_log,
            clipboard_store,
        };
        let result = (tool.handler)(
            &json!({"title": "Integration test ticket", "priority": "high"}),
            &mut ctx,
        )
        .expect("tool invocation should succeed");

        assert!(result["ticket_id"].is_string());
        assert_eq!(result["title"], "Integration test ticket");
        assert_eq!(result["priority"], "high");
        assert_eq!(result["status"], "open");

        // Unload the extension
        mgr.unload_extension("my-app", &mut registry)
            .expect("should unload");
        assert!(mgr.loaded_extensions().is_empty());
        assert!(registry.lookup("my-app", "my_app.create_ticket").is_none());
    }

    /// Timeout test: host hangs and invocation times out.
    ///
    /// Requires Deno to be installed. Skipped if `deno` is not on PATH.
    #[test]
    fn timeout_host_hangs_invocation_times_out() {
        // Check if Deno is available
        if Command::new("deno").arg("--version").output().is_err() {
            eprintln!("skipping timeout test: deno not found");
            return;
        }

        // Create a hanging extension: a Deno script that reads stdin but never responds
        let dir = tempfile::tempdir().unwrap();
        let hanging_script = "// Read stdin but never write a response\nawait new Promise(() => {});\n";
        std::fs::write(dir.path().join("hanging.ts"), hanging_script).unwrap();

        // Spawn a Deno process that hangs
        let mut child = Command::new("deno")
            .arg("run")
            .arg(dir.path().join("hanging.ts").to_string_lossy().as_ref())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("should spawn deno");

        let child_stdin = child.stdin.take().unwrap();
        let child_stdout = child.stdout.take().unwrap();

        let stdin: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(child_stdin)));
        let stdout: Arc<Mutex<BufReader<Box<dyn Read + Send>>>> =
            Arc::new(Mutex::new(BufReader::new(Box::new(child_stdout))));
        let next_id = Arc::new(AtomicU64::new(1));

        // Use a very short timeout (100ms)
        let result = rpc_call(&stdin, &stdout, &next_id, "initialize", None, 100);

        // The call should fail with a timeout or EOF error
        assert!(result.is_err(), "should timeout or get EOF");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("timeout") || err_msg.contains("EOF") || err_msg.contains("closed"),
            "error should indicate timeout or closed connection, got: {err_msg}"
        );

        // Clean up
        let _ = child.kill();
        let _ = child.wait();
    }
}
