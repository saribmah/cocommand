import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import "./App.css";
import { hideWindow } from "./lib/ipc";
import * as backend from "./lib/backend";

/** Panel tabs for the workspace view */
const TABS = ["workspace", "apps", "tools"];

function App() {
  const [input, setInput] = useState("");
  const [result, setResult] = useState("");
  const [backendStatus, setBackendStatus] = useState("checking");
  const [activeTab, setActiveTab] = useState("workspace");

  // Workspace state
  const [snapshot, setSnapshot] = useState(null);
  const [lifecycleMessage, setLifecycleMessage] = useState(null);
  const [isArchived, setIsArchived] = useState(false);
  const [isSoftReset, setIsSoftReset] = useState(false);

  // Apps and tools
  const [apps, setApps] = useState([]);
  const [tools, setTools] = useState([]);
  const [loadingApps, setLoadingApps] = useState({});

  // Tool runner state
  const [selectedTool, setSelectedTool] = useState(null);
  const [toolInputs, setToolInputs] = useState("{}");
  const [toolResult, setToolResult] = useState(null);

  const resizeFrameRef = useRef(0);
  const lastHeightRef = useRef(140);
  const inputRef = useRef(null);

  // Refresh workspace snapshot and tools
  const refreshWorkspace = useCallback(async () => {
    try {
      const snapshotRes = await backend.getSnapshot();
      if (snapshotRes.status === "ok" && snapshotRes.snapshot) {
        setSnapshot(snapshotRes.snapshot);
        setLifecycleMessage(snapshotRes.message || null);
        setIsArchived(snapshotRes.archived || false);
        setIsSoftReset(snapshotRes.soft_reset || false);
      }

      const toolsRes = await backend.listTools();
      setTools(toolsRes);
    } catch (error) {
      console.error("Failed to refresh workspace:", error);
    }
  }, []);

  // Load apps list
  const loadApps = useCallback(async () => {
    try {
      const appsRes = await backend.listApps();
      setApps(appsRes);
    } catch (error) {
      console.error("Failed to load apps:", error);
    }
  }, []);

  // Submit a command
  async function submitCommand() {
    try {
      const trimmed = input.trim();
      if (!trimmed) {
        setResult("Type a command to get started.");
        return;
      }

      setResult("Processing...");
      const response = await backend.submitCommand(trimmed);

      if (response.status !== "ok") {
        setResult(response.message ?? "Unable to run the command.");
        return;
      }

      setInput("");
      const msg = response.result?.message ?? "Command executed.";
      const phase = response.phase ? ` [${response.phase}]` : "";
      const turns = response.turns_used ? ` (${response.turns_used} turns)` : "";
      setResult(`${msg}${phase}${turns}`);

      // Refresh workspace state after command
      await refreshWorkspace();
    } catch (error) {
      setResult(`Error: ${error}`);
    }
  }

  // Open an app
  async function handleOpenApp(appId) {
    setLoadingApps((prev) => ({ ...prev, [appId]: true }));
    try {
      const res = await backend.openApp(appId);
      if (res.status === "ok" && res.snapshot) {
        setSnapshot(res.snapshot);
        setIsArchived(res.archived || false);
        await refreshWorkspace();
      } else if (res.archived) {
        setIsArchived(true);
        setResult(res.message || "Workspace is archived. Restore it first.");
      } else {
        setResult(res.message || "Failed to open app.");
      }
    } catch (error) {
      setResult(`Error opening app: ${error}`);
    } finally {
      setLoadingApps((prev) => ({ ...prev, [appId]: false }));
    }
  }

  // Close an app
  async function handleCloseApp(appId) {
    setLoadingApps((prev) => ({ ...prev, [appId]: true }));
    try {
      const res = await backend.closeApp(appId);
      if (res.status === "ok" && res.snapshot) {
        setSnapshot(res.snapshot);
        await refreshWorkspace();
      } else {
        setResult(res.message || "Failed to close app.");
      }
    } catch (error) {
      setResult(`Error closing app: ${error}`);
    } finally {
      setLoadingApps((prev) => ({ ...prev, [appId]: false }));
    }
  }

  // Focus an app
  async function handleFocusApp(appId) {
    try {
      const res = await backend.focusApp(appId);
      if (res.status === "ok" && res.snapshot) {
        setSnapshot(res.snapshot);
      } else {
        setResult(res.message || "Failed to focus app.");
      }
    } catch (error) {
      setResult(`Error focusing app: ${error}`);
    }
  }

  // Restore archived workspace
  async function handleRestoreWorkspace() {
    try {
      setResult("Restoring workspace...");
      const res = await backend.restoreWorkspace();
      if (res.status === "ok" && res.snapshot) {
        setSnapshot(res.snapshot);
        setIsArchived(false);
        setIsSoftReset(false);
        setLifecycleMessage(null);
        setResult("Workspace restored.");
        await refreshWorkspace();
      } else {
        setResult(res.message || "Failed to restore workspace.");
      }
    } catch (error) {
      setResult(`Error restoring workspace: ${error}`);
    }
  }

  // Execute a tool
  async function handleExecuteTool() {
    if (!selectedTool) return;
    try {
      let inputs = {};
      try {
        inputs = JSON.parse(toolInputs);
      } catch (e) {
        setToolResult({ status: "error", message: "Invalid JSON inputs" });
        return;
      }

      setToolResult({ status: "pending", message: "Executing..." });
      const res = await backend.executeTool(selectedTool.id, inputs);
      setToolResult(res);
      await refreshWorkspace();
    } catch (error) {
      setToolResult({ status: "error", message: `Error: ${error}` });
    }
  }

  // Health check polling
  useEffect(() => {
    let active = true;
    const check = () => {
      backend.getHealth()
        .then((data) => {
          if (!active) return;
          const next = data?.status === "ok" ? "ok" : "error";
          setBackendStatus(next);
          if (next === "ok") {
            clearInterval(timer);
            // Initial data load on connect
            loadApps();
            refreshWorkspace();
          }
        })
        .catch(() => {
          if (!active) return;
          setBackendStatus("error");
        });
    };
    const timer = setInterval(check, 800);
    check();
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, [loadApps, refreshWorkspace]);

  // Window focus handling
  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const windowHandle = getCurrentWindow();
    let unlisten;
    windowHandle
      .onFocusChanged(({ payload: focused }) => {
        if (focused) {
          requestAnimationFrame(() => {
            inputRef.current?.focus();
            inputRef.current?.select();
          });
          // Refresh on focus
          refreshWorkspace();
        } else {
          windowHandle.hide().catch(() => {});
        }
      })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {});
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [refreshWorkspace]);

  // Dynamic window resizing
  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;

    const baseHeight = 140;
    const tabsHeight = 50;
    const contentHeight = activeTab ? 260 : 0;
    const resultHeight = result ? 70 : 0;
    const nextHeight = baseHeight + tabsHeight + contentHeight + resultHeight;

    const windowHandle = getCurrentWindow();
    const startHeight = lastHeightRef.current;
    const targetHeight = nextHeight;
    const startTime = performance.now();
    const duration = 180;

    cancelAnimationFrame(resizeFrameRef.current);

    const step = (now) => {
      const elapsed = Math.min(now - startTime, duration);
      const progress = elapsed / duration;
      const eased = 1 - Math.pow(1 - progress, 3);
      const currentHeight = Math.round(
        startHeight + (targetHeight - startHeight) * eased
      );
      lastHeightRef.current = currentHeight;
      windowHandle
        .setSize(new LogicalSize(720, currentHeight))
        .catch(() => {});
      if (elapsed < duration) {
        resizeFrameRef.current = requestAnimationFrame(step);
      }
    };

    resizeFrameRef.current = requestAnimationFrame(step);
    return () => cancelAnimationFrame(resizeFrameRef.current);
  }, [result, activeTab]);

  // Check if app is open
  const isAppOpen = (appId) => {
    return snapshot?.open_apps?.some((app) => app.id === appId) || false;
  };

  // Check if app is focused
  const isAppFocused = (appId) => {
    return snapshot?.focused_app === appId;
  };

  return (
    <main className="container">
      {/* Command input */}
      <form
        className="command"
        onSubmit={(e) => {
          e.preventDefault();
          submitCommand();
        }}
      >
        <div className="command-badge">coco</div>
        <input
          id="command-input"
          ref={inputRef}
          value={input}
          onChange={(e) => setInput(e.currentTarget.value)}
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              if (input.trim()) {
                setInput("");
                return;
              }
              hideWindow().catch(() => {});
            }
          }}
          placeholder="Ask coco to do something..."
        />
        <button type="submit">Run</button>
      </form>

      {/* Tabs */}
      <div className="tabs">
        {TABS.map((tab) => (
          <button
            key={tab}
            className={`tab ${activeTab === tab ? "active" : ""}`}
            onClick={() => setActiveTab(tab)}
          >
            {tab.charAt(0).toUpperCase() + tab.slice(1)}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div className="tab-content">
        {/* Workspace tab */}
        {activeTab === "workspace" && (
          <div className="workspace-panel">
            {isArchived && (
              <div className="lifecycle-banner archived">
                <span>Workspace is archived (inactive &gt;7 days)</span>
                <button onClick={handleRestoreWorkspace}>Restore</button>
              </div>
            )}
            {isSoftReset && !isArchived && (
              <div className="lifecycle-banner soft-reset">
                <span>Workspace was soft-reset due to inactivity</span>
              </div>
            )}
            {lifecycleMessage && !isArchived && !isSoftReset && (
              <div className="lifecycle-banner info">
                <span>{lifecycleMessage}</span>
              </div>
            )}
            {snapshot ? (
              <div className="snapshot-info">
                <div className="snapshot-row">
                  <span className="snapshot-label">Staleness</span>
                  <span className={`staleness-badge ${snapshot.staleness}`}>
                    {snapshot.staleness}
                  </span>
                </div>
                <div className="snapshot-row">
                  <span className="snapshot-label">Focused App</span>
                  <span className="snapshot-value">
                    {snapshot.focused_app || "None"}
                  </span>
                </div>
                <div className="snapshot-row">
                  <span className="snapshot-label">Open Apps</span>
                  <span className="snapshot-value">
                    {snapshot.open_apps?.length || 0}
                  </span>
                </div>
                {snapshot.open_apps?.length > 0 && (
                  <div className="open-apps-list">
                    {snapshot.open_apps.map((app) => (
                      <div
                        key={app.id}
                        className={`open-app-item ${
                          isAppFocused(app.id) ? "focused" : ""
                        }`}
                      >
                        <span className="app-id">{app.id}</span>
                        <span className="app-summary">{app.summary}</span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ) : (
              <div className="panel-empty">No workspace data</div>
            )}
          </div>
        )}

        {/* Apps tab */}
        {activeTab === "apps" && (
          <div className="apps-panel">
            {apps.length === 0 ? (
              <div className="panel-empty">No apps available</div>
            ) : (
              <ul className="panel-list">
                {apps.map((app) => (
                  <li key={app.id}>
                    <div className="app-row">
                      <div className="app-info">
                        <span className="app-name">{app.name}</span>
                        <small className="app-desc">{app.description}</small>
                      </div>
                      <div className="app-actions">
                        {isAppOpen(app.id) ? (
                          <>
                            {!isAppFocused(app.id) && (
                              <button
                                className="btn-sm btn-focus"
                                onClick={() => handleFocusApp(app.id)}
                              >
                                Focus
                              </button>
                            )}
                            <button
                              className="btn-sm btn-close"
                              onClick={() => handleCloseApp(app.id)}
                              disabled={loadingApps[app.id]}
                            >
                              {loadingApps[app.id] ? "..." : "Close"}
                            </button>
                          </>
                        ) : (
                          <button
                            className="btn-sm btn-open"
                            onClick={() => handleOpenApp(app.id)}
                            disabled={loadingApps[app.id] || isArchived}
                          >
                            {loadingApps[app.id] ? "..." : "Open"}
                          </button>
                        )}
                      </div>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}

        {/* Tools tab */}
        {activeTab === "tools" && (
          <div className="tools-panel">
            {tools.length === 0 ? (
              <div className="panel-empty">
                No tools available. Open an app first.
              </div>
            ) : (
              <div className="tools-content">
                <div className="tools-list">
                  {tools.map((tool) => (
                    <button
                      key={tool.id}
                      className={`tool-item ${
                        selectedTool?.id === tool.id ? "active" : ""
                      }`}
                      onClick={() => {
                        setSelectedTool(tool);
                        setToolInputs("{}");
                        setToolResult(null);
                      }}
                    >
                      <span className="tool-name">{tool.name}</span>
                      <small className="tool-desc">{tool.description}</small>
                    </button>
                  ))}
                </div>
                {selectedTool && (
                  <div className="tool-runner">
                    <div className="tool-runner-header">
                      <span className="tool-id">{selectedTool.id}</span>
                    </div>
                    <textarea
                      className="tool-inputs"
                      value={toolInputs}
                      onChange={(e) => setToolInputs(e.target.value)}
                      placeholder='{"key": "value"}'
                      rows={3}
                    />
                    <button
                      className="btn-execute"
                      onClick={handleExecuteTool}
                      disabled={isArchived}
                    >
                      Execute
                    </button>
                    {toolResult && (
                      <div
                        className={`tool-result ${
                          toolResult.status === "ok" ? "success" : "error"
                        }`}
                      >
                        <span className="tool-result-status">
                          {toolResult.status}
                        </span>
                        <p>{toolResult.message}</p>
                      </div>
                    )}
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>

      {/* Result display */}
      {result && (
        <div className="result">
          <span>Result</span>
          <p>{result}</p>
        </div>
      )}

      {/* Backend status */}
      {backendStatus !== "checking" && (
        <div
          className={
            backendStatus === "ok"
              ? "backend-banner backend-banner-ok"
              : "backend-banner backend-banner-error"
          }
        >
          {backendStatus === "ok"
            ? "Backend connected"
            : "Backend not reachable. Ensure the server is running."}
        </div>
      )}
    </main>
  );
}

export default App;
