import "./SettingsView.css";
import { useEffect, useState } from "react";
import { useServerStore } from "../../state/server";
import { useAiStore } from "../../state/ai";
import { hideSettingsWindow } from "../../lib/ipc";

export function SettingsView() {
  const serverInfo = useServerStore((state) => state.info);
  const aiSettings = useAiStore((state) => state.settings);
  const aiLoaded = useAiStore((state) => state.isLoaded);
  const aiError = useAiStore((state) => state.error);
  const fetchAiSettings = useAiStore((state) => state.fetchSettings);
  const updateAiSettings = useAiStore((state) => state.updateSettings);
  const [tab, setTab] = useState<"overview" | "workspace" | "ai">("overview");
  const [aiForm, setAiForm] = useState({
    provider: "openai-compatible",
    base_url: "",
    model: "",
    system_prompt: "",
    temperature: "0.7",
    max_output_tokens: "80000",
    max_steps: "8",
    api_key: "",
  });
  const [aiSaving, setAiSaving] = useState(false);
  const [aiToast, setAiToast] = useState<null | "success" | "error">(null);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        hideSettingsWindow();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  useEffect(() => {
    if (!serverInfo) return;
    if (!aiLoaded) {
      fetchAiSettings();
    }
  }, [serverInfo, aiLoaded, fetchAiSettings]);

  useEffect(() => {
    if (!aiSettings) return;
    setAiForm({
      provider: aiSettings.provider,
      base_url: aiSettings.base_url,
      model: aiSettings.model,
      system_prompt: aiSettings.system_prompt,
      temperature: String(aiSettings.temperature ?? 0.7),
      max_output_tokens: String(aiSettings.max_output_tokens ?? 80000),
      max_steps: String(aiSettings.max_steps ?? 8),
      api_key: "",
    });
  }, [aiSettings]);

  const handleAiChange = (field: keyof typeof aiForm, value: string) => {
    setAiForm((prev) => ({ ...prev, [field]: value }));
  };

  const saveAiSettings = async () => {
    setAiSaving(true);
    setAiToast(null);
    try {
      await updateAiSettings({
        provider: aiForm.provider,
        base_url: aiForm.base_url,
        model: aiForm.model,
        system_prompt: aiForm.system_prompt,
        temperature: Number(aiForm.temperature),
        max_output_tokens: Number(aiForm.max_output_tokens),
        max_steps: Number(aiForm.max_steps),
        api_key: aiForm.api_key.trim().length ? aiForm.api_key.trim() : undefined,
      });
      setAiForm((prev) => ({ ...prev, api_key: "" }));
      setAiToast("success");
    } finally {
      setAiSaving(false);
    }
  };

  useEffect(() => {
    if (!aiToast) return;
    const timer = window.setTimeout(() => setAiToast(null), 2500);
    return () => window.clearTimeout(timer);
  }, [aiToast]);

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
              <div className="settings-row settings-row--input">
                <div className="settings-row__title">Provider</div>
                <div className="settings-row__value">
                  <input
                    className="settings-input"
                    value={aiForm.provider}
                    onChange={(event) => handleAiChange("provider", event.target.value)}
                    placeholder="openai-compatible"
                  />
                </div>
              </div>
              <div className="settings-row settings-row--input">
                <div className="settings-row__title">Base URL</div>
                <div className="settings-row__value">
                  <input
                    className="settings-input"
                    value={aiForm.base_url}
                    onChange={(event) => handleAiChange("base_url", event.target.value)}
                    placeholder="https://api.openai.com/v1"
                  />
                </div>
              </div>
              <div className="settings-row settings-row--input">
                <div className="settings-row__title">Model</div>
                <div className="settings-row__value">
                  <input
                    className="settings-input"
                    value={aiForm.model}
                    onChange={(event) => handleAiChange("model", event.target.value)}
                    placeholder="gpt-4o-mini"
                  />
                </div>
              </div>
              <div className="settings-row settings-row--input">
                <div className="settings-row__title">System prompt</div>
                <div className="settings-row__value">
                  <textarea
                    className="settings-input settings-input--textarea"
                    value={aiForm.system_prompt}
                    onChange={(event) => handleAiChange("system_prompt", event.target.value)}
                    placeholder="You are Cocommand, a helpful command assistant."
                  />
                </div>
              </div>
              <div className="settings-row settings-row--input">
                <div className="settings-row__title">Temperature</div>
                <div className="settings-row__value">
                  <input
                    className="settings-input"
                    value={aiForm.temperature}
                    onChange={(event) => handleAiChange("temperature", event.target.value)}
                    type="number"
                    step="0.1"
                    min="0"
                    max="2"
                  />
                </div>
              </div>
              <div className="settings-row settings-row--input">
                <div className="settings-row__title">Max output tokens</div>
                <div className="settings-row__value">
                  <input
                    className="settings-input"
                    value={aiForm.max_output_tokens}
                    onChange={(event) =>
                      handleAiChange("max_output_tokens", event.target.value)
                    }
                    type="number"
                    min="256"
                  />
                </div>
              </div>
              <div className="settings-row settings-row--input">
                <div className="settings-row__title">Max steps</div>
                <div className="settings-row__value">
                  <input
                    className="settings-input"
                    value={aiForm.max_steps}
                    onChange={(event) => handleAiChange("max_steps", event.target.value)}
                    type="number"
                    min="1"
                  />
                </div>
              </div>
              <div className="settings-row settings-row--input">
                <div className="settings-row__title">API key</div>
                <div className="settings-row__value">
                  <input
                    className="settings-input"
                    value={aiForm.api_key}
                    onChange={(event) => handleAiChange("api_key", event.target.value)}
                    placeholder={aiSettings?.has_api_key ? "Configured" : "sk-..."}
                    type="password"
                  />
                </div>
              </div>
            </div>
            {serverInfo && aiError && (
              <div className="settings-error">{aiError}</div>
            )}
            {aiToast === "success" && (
              <div className="settings-toast">Settings saved</div>
            )}
            <div className="settings-actions">
              <button
                className="settings-button"
                type="button"
                onClick={saveAiSettings}
                disabled={aiSaving}
              >
                {aiSaving ? "Saving..." : "Save changes"}
              </button>
            </div>
          </div>
        )}
      </section>
    </main>
  );
}
