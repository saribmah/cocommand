import "./SettingsView.css";
import { useEffect, useState } from "react";
import { useServerStore } from "../../state/server";
import { hideSettingsWindow } from "../../lib/ipc";

export function SettingsView() {
  const serverInfo = useServerStore((state) => state.info);
  const [tab, setTab] = useState<"overview" | "workspace" | "ai">("overview");

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        hideSettingsWindow();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  return (
    <main className="settings-shell">
      <header className="settings-bar">
        <div className="settings-bar__brand">
          <div className="settings-logo">CC</div>
          <div>
            <p className="settings-kicker">Cocommand</p>
            <h1>Settings</h1>
          </div>
        </div>
        <nav className="settings-tabs">
          <button
            className={`settings-tab ${tab === "overview" ? "is-active" : ""}`}
            onClick={() => setTab("overview")}
            type="button"
          >
            Overview
          </button>
          <button
            className={`settings-tab ${tab === "workspace" ? "is-active" : ""}`}
            onClick={() => setTab("workspace")}
            type="button"
          >
            Workspace
          </button>
          <button
            className={`settings-tab ${tab === "ai" ? "is-active" : ""}`}
            onClick={() => setTab("ai")}
            type="button"
          >
            AI
          </button>
        </nav>
        <div className="settings-bar__status">
          <span
            className={`status-dot ${serverInfo ? "is-live" : "is-off"}`}
            aria-hidden="true"
          />
          <div>
            <p className="status-title">
              {serverInfo ? "Server running" : "Server offline"}
            </p>
            <p className="status-meta">{serverInfo?.addr ?? "Not connected"}</p>
          </div>
        </div>
      </header>

      <section className="settings-section">
        {tab === "overview" && (
          <div className="settings-panel">
            <div className="settings-section-title">
              <h2>General</h2>
              <p className="settings-muted">
                Core configuration for this device.
              </p>
            </div>
            <div className="settings-list">
              <div className="settings-row">
                <div className="settings-row__title">Server address</div>
                <div className="settings-row__value">
                  {serverInfo?.addr ?? "Not connected"}
                </div>
              </div>
              <div className="settings-row">
                <div className="settings-row__title">Workspace path</div>
                <div className="settings-row__value">
                  {serverInfo?.workspace_dir ?? "Unknown"}
                </div>
              </div>
              <div className="settings-row">
                <div className="settings-row__title">Shortcuts</div>
                <div className="settings-row__value">
                  <span className="settings-pill">/settings</span>
                  <span className="settings-pill">Esc</span>
                  <span className="settings-pill">/help</span>
                </div>
              </div>
            </div>
          </div>
        )}

        {tab === "workspace" && (
          <div className="settings-panel">
            <div className="settings-section-title">
              <h2>Workspace</h2>
              <p className="settings-muted">
                Configure where Cocommand stores sessions and files.
              </p>
            </div>
            <div className="settings-list">
              <div className="settings-row">
                <div className="settings-row__title">Workspace path</div>
                <div className="settings-row__value">
                  {serverInfo?.workspace_dir ?? "Unknown"}
                </div>
              </div>
            </div>
            <div className="settings-placeholder">
              Workspace controls are coming next.
            </div>
          </div>
        )}

        {tab === "ai" && (
          <div className="settings-panel">
            <div className="settings-section-title">
              <h2>AI</h2>
              <p className="settings-muted">Configure models and providers</p>
            </div>
            <div className="settings-list">
              <div className="settings-row">
                <div className="settings-row__title">Provider</div>
                <div className="settings-row__value">OpenAI compatible</div>
              </div>
            </div>
            <div className="settings-placeholder">AI settings UI coming next.</div>
          </div>
        )}
      </section>
    </main>
  );
}
