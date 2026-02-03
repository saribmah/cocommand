import "./OnboardingView.css";
import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useServerStore } from "../../state/server";
import { useOnboardingStore } from "../../state/onboarding";
import { useWorkspaceStore } from "../../state/workspace";
import { useAiStore } from "../../state/ai";
import { usePermissionsStore } from "../../state/permissions";
import { setWorkspaceDir } from "../../lib/ipc";

type StepId =
  | "welcome"
  | "workspace"
  | "theme"
  | "ai"
  | "permissions"
  | "finish";

const steps: { id: StepId; title: string; subtitle: string }[] = [
  {
    id: "welcome",
    title: "Welcome",
    subtitle: "Set up your workspace, AI, and permissions in a few steps.",
  },
  {
    id: "workspace",
    title: "Workspace",
    subtitle: "Pick where Cocommand stores sessions and files.",
  },
  {
    id: "theme",
    title: "Theme",
    subtitle: "Name the workspace and choose a look you like.",
  },
  {
    id: "ai",
    title: "AI settings",
    subtitle: "Configure the model and provider you want to use.",
  },
  {
    id: "permissions",
    title: "Permissions",
    subtitle: "Enable macOS permissions required for automation.",
  },
  {
    id: "finish",
    title: "Finish",
    subtitle: "You are ready to start commanding.",
  },
];

const accentOptions = [
  { id: "copper", label: "Copper", color: "#f46a4b" },
  { id: "sunset", label: "Sunset", color: "#f0a15c" },
  { id: "ember", label: "Ember", color: "#d25b7a" },
  { id: "sea", label: "Sea", color: "#3ea7a0" },
  { id: "electric", label: "Electric", color: "#4aa6ff" },
];

const themeModes = [
  { id: "system", label: "System" },
  { id: "light", label: "Light" },
  { id: "dark", label: "Dark" },
];

export function OnboardingView() {
  const serverInfo = useServerStore((state) => state.info);
  const setServerInfo = useServerStore((state) => state.setInfo);
  const onboardingUpdate = useOnboardingStore((state) => state.updateStatus);
  const workspaceSettings = useWorkspaceStore((state) => state.settings);
  const workspaceLoaded = useWorkspaceStore((state) => state.isLoaded);
  const fetchWorkspace = useWorkspaceStore((state) => state.fetchSettings);
  const updateWorkspace = useWorkspaceStore((state) => state.updateSettings);
  const aiSettings = useAiStore((state) => state.settings);
  const aiLoaded = useAiStore((state) => state.isLoaded);
  const fetchAiSettings = useAiStore((state) => state.fetchSettings);
  const updateAiSettings = useAiStore((state) => state.updateSettings);
  const permissions = usePermissionsStore((state) => state.permissions);
  const permissionsLoaded = usePermissionsStore((state) => state.isLoaded);
  const fetchPermissions = usePermissionsStore((state) => state.fetchStatus);
  const openPermission = usePermissionsStore((state) => state.openPermission);
  const [stepIndex, setStepIndex] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [workspaceType, setWorkspaceType] = useState<"local" | "remote">("local");
  const [workspacePath, setWorkspacePath] = useState("");
  const [workspaceName, setWorkspaceName] = useState("");
  const [themeMode, setThemeMode] = useState("system");
  const [themeAccent, setThemeAccent] = useState("copper");
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

  const stepId = steps[stepIndex]?.id ?? "welcome";

  useEffect(() => {
    if (!serverInfo) return;
    if (!workspaceLoaded) fetchWorkspace();
    if (!aiLoaded) fetchAiSettings();
    if (!permissionsLoaded) fetchPermissions();
  }, [
    serverInfo,
    workspaceLoaded,
    aiLoaded,
    permissionsLoaded,
    fetchWorkspace,
    fetchAiSettings,
    fetchPermissions,
  ]);

  useEffect(() => {
    if (!serverInfo) return;
    setWorkspacePath(serverInfo.workspace_dir);
  }, [serverInfo]);

  useEffect(() => {
    if (!workspaceSettings) return;
    setWorkspaceName(workspaceSettings.name);
    setThemeMode(workspaceSettings.theme.mode);
    setThemeAccent(workspaceSettings.theme.accent);
  }, [workspaceSettings]);

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

  useEffect(() => {
    if (stepId !== "permissions") return;
    const timer = window.setInterval(() => {
      fetchPermissions();
    }, 4500);
    return () => window.clearInterval(timer);
  }, [stepId, fetchPermissions]);

  const allPermissionsGranted = useMemo(() => {
    if (!permissions.length) return false;
    return permissions.every((permission) => !permission.required || permission.granted);
  }, [permissions]);

  useEffect(() => {
    if (stepId !== "permissions") return;
    if (allPermissionsGranted && !busy) {
      setStepIndex((prev) => Math.min(prev + 1, steps.length - 1));
    }
  }, [stepId, allPermissionsGranted, busy]);

  const handleAiChange = (field: keyof typeof aiForm, value: string) => {
    setAiForm((prev) => ({ ...prev, [field]: value }));
  };

  const handlePickWorkspace = async () => {
    setError(null);
    try {
      const result = await open({
        title: "Choose a workspace folder",
        directory: true,
        multiple: false,
      });
      if (typeof result === "string") {
        setWorkspacePath(result);
      }
    } catch (err) {
      setError(String(err));
    }
  };

  const handleOpenPermission = async (id: string) => {
    setError(null);
    try {
      await openPermission(id);
      await fetchPermissions();
    } catch (err) {
      setError(String(err));
    }
  };

  const handleNext = async () => {
    setError(null);
    if (busy) return;
    if (stepId === "welcome") {
      setStepIndex(stepIndex + 1);
      return;
    }

    if (stepId === "workspace") {
      if (workspaceType === "remote") {
        setError("Remote workspaces are coming soon.");
        return;
      }
      if (!workspacePath.trim()) {
        setError("Please select a workspace folder.");
        return;
      }
      if (!serverInfo) {
        setError("Server unavailable.");
        return;
      }
      setBusy(true);
      try {
        if (workspacePath !== serverInfo.workspace_dir) {
          const newAddr = await setWorkspaceDir(workspacePath);
          setServerInfo({ addr: newAddr, workspace_dir: workspacePath });
          await fetchWorkspace();
          await fetchAiSettings();
        }
        setStepIndex(stepIndex + 1);
      } catch (err) {
        setError(String(err));
      } finally {
        setBusy(false);
      }
      return;
    }

    if (stepId === "theme") {
      if (!workspaceName.trim()) {
        setError("Workspace name is required.");
        return;
      }
      setBusy(true);
      try {
        await updateWorkspace({
          name: workspaceName.trim(),
          theme_mode: themeMode,
          theme_accent: themeAccent,
        });
        setStepIndex(stepIndex + 1);
      } catch (err) {
        setError(String(err));
      } finally {
        setBusy(false);
      }
      return;
    }

    if (stepId === "ai") {
      if (!aiForm.provider.trim()) {
        setError("Provider is required.");
        return;
      }
      if (!aiForm.base_url.trim()) {
        setError("Base URL is required.");
        return;
      }
      if (!aiForm.model.trim()) {
        setError("Model is required.");
        return;
      }
      const needsKey = !aiSettings?.has_api_key;
      if (needsKey && !aiForm.api_key.trim()) {
        setError("API key is required to continue.");
        return;
      }
      setBusy(true);
      try {
        await updateAiSettings({
          provider: aiForm.provider.trim(),
          base_url: aiForm.base_url.trim(),
          model: aiForm.model.trim(),
          system_prompt: aiForm.system_prompt.trim(),
          temperature: Number(aiForm.temperature),
          max_output_tokens: Number(aiForm.max_output_tokens),
          max_steps: Number(aiForm.max_steps),
          api_key: aiForm.api_key.trim().length ? aiForm.api_key.trim() : undefined,
        });
        setAiForm((prev) => ({ ...prev, api_key: "" }));
        setStepIndex(stepIndex + 1);
      } catch (err) {
        setError(String(err));
      } finally {
        setBusy(false);
      }
      return;
    }

    if (stepId === "permissions") {
      if (!allPermissionsGranted) {
        setError("All required permissions must be enabled to continue.");
        return;
      }
      setStepIndex(stepIndex + 1);
      return;
    }

    if (stepId === "finish") {
      setBusy(true);
      try {
        await onboardingUpdate({ completed: true, version: "1.0" });
      } catch (err) {
        setError(String(err));
      } finally {
        setBusy(false);
      }
      return;
    }
  };

  const handleBack = () => {
    if (busy) return;
    if (stepIndex > 0) setStepIndex(stepIndex - 1);
  };

  return (
    <main className="onboarding-shell">
      <div className="onboarding-bg" />
      <div className="onboarding-layout">
        <aside className="onboarding-rail">
          <div className="onboarding-brand">
            <div className="onboarding-logo">CC</div>
            <div>
              <p className="onboarding-kicker">Cocommand</p>
              <h1>Onboarding</h1>
            </div>
          </div>
          <div className="onboarding-steps">
            {steps.map((step, index) => (
              <div
                className={`onboarding-step ${
                  index === stepIndex ? "is-active" : index < stepIndex ? "is-done" : ""
                }`}
                key={step.id}
              >
                <div className="onboarding-step__marker">
                  {index < stepIndex ? "OK" : index + 1}
                </div>
                <div>
                  <p className="onboarding-step__title">{step.title}</p>
                  <p className="onboarding-step__subtitle">{step.subtitle}</p>
                </div>
              </div>
            ))}
          </div>
        </aside>

        <section className="onboarding-panel">
          <div className="onboarding-panel__header">
            <h2>{steps[stepIndex]?.title}</h2>
            <p>{steps[stepIndex]?.subtitle}</p>
          </div>

          {stepId === "welcome" && (
            <div className="onboarding-card">
              <h3>Let's set up your command center</h3>
              <p>
                Cocommand uses a workspace to store sessions, models, and automation
                preferences. We'll also enable macOS permissions so commands run without
                interruptions.
              </p>
              <div className="onboarding-highlight">
                <div>
                  <h4>Fast setup</h4>
                  <p>Most teams finish in under two minutes.</p>
                </div>
                <div>
                  <h4>Secure by default</h4>
                  <p>Your keys and settings stay on this device.</p>
                </div>
              </div>
            </div>
          )}

          {stepId === "workspace" && (
            <div className="onboarding-card">
              <div className="onboarding-grid">
                <button
                  className={`onboarding-choice ${
                    workspaceType === "local" ? "is-selected" : ""
                  }`}
                  type="button"
                  onClick={() => setWorkspaceType("local")}
                >
                  <h3>Local workspace</h3>
                  <p>Store data on this Mac for maximum speed.</p>
                </button>
                <button
                  className="onboarding-choice is-disabled"
                  type="button"
                  disabled
                >
                  <h3>Remote workspace</h3>
                  <p>Connect to a server in the cloud. Coming soon.</p>
                </button>
              </div>

              <div className="onboarding-field">
                <label>Workspace folder</label>
                <div className="onboarding-field__row">
                  <input value={workspacePath} readOnly />
                  <button type="button" onClick={handlePickWorkspace}>
                    Choose
                  </button>
                </div>
                <span className="onboarding-help">
                  This folder will store sessions, history, and extensions.
                </span>
              </div>
            </div>
          )}

          {stepId === "theme" && (
            <div className="onboarding-card">
              <div className="onboarding-field">
                <label>Workspace name</label>
                <input
                  value={workspaceName}
                  onChange={(event) => setWorkspaceName(event.target.value)}
                  placeholder="My workspace"
                />
              </div>
              <div className="onboarding-field">
                <label>Theme mode</label>
                <div className="onboarding-chip-row">
                  {themeModes.map((mode) => (
                    <button
                      key={mode.id}
                      type="button"
                      className={`onboarding-chip ${
                        themeMode === mode.id ? "is-selected" : ""
                      }`}
                      onClick={() => setThemeMode(mode.id)}
                    >
                      {mode.label}
                    </button>
                  ))}
                </div>
              </div>
              <div className="onboarding-field">
                <label>Accent color</label>
                <div className="onboarding-accent-row">
                  {accentOptions.map((accent) => (
                    <button
                      key={accent.id}
                      type="button"
                      className={`onboarding-accent ${
                        themeAccent === accent.id ? "is-selected" : ""
                      }`}
                      onClick={() => setThemeAccent(accent.id)}
                    >
                      <span style={{ background: accent.color }} />
                      {accent.label}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          )}

          {stepId === "ai" && (
            <div className="onboarding-card">
              <div className="onboarding-field">
                <label>Provider</label>
                <input
                  value={aiForm.provider}
                  onChange={(event) => handleAiChange("provider", event.target.value)}
                  placeholder="openai-compatible"
                />
              </div>
              <div className="onboarding-field">
                <label>Base URL</label>
                <input
                  value={aiForm.base_url}
                  onChange={(event) => handleAiChange("base_url", event.target.value)}
                  placeholder="https://api.openai.com/v1"
                />
              </div>
              <div className="onboarding-field">
                <label>Model</label>
                <input
                  value={aiForm.model}
                  onChange={(event) => handleAiChange("model", event.target.value)}
                  placeholder="gpt-4o-mini"
                />
              </div>
              <div className="onboarding-field">
                <label>System prompt</label>
                <textarea
                  value={aiForm.system_prompt}
                  onChange={(event) => handleAiChange("system_prompt", event.target.value)}
                  placeholder="You are Cocommand, a helpful command assistant."
                />
              </div>
              <div className="onboarding-field">
                <label>API key</label>
                <input
                  type="password"
                  value={aiForm.api_key}
                  onChange={(event) => handleAiChange("api_key", event.target.value)}
                  placeholder={aiSettings?.has_api_key ? "Configured" : "sk-..."}
                />
                <span className="onboarding-help">
                  Required to continue unless already configured.
                </span>
              </div>
            </div>
          )}

          {stepId === "permissions" && (
            <div className="onboarding-card">
              <div className="onboarding-permissions">
                {permissions.length === 0 ? (
                  <div className="onboarding-note">Checking permissions...</div>
                ) : (
                  permissions.map((permission) => (
                    <div key={permission.id} className="onboarding-permission">
                      <div>
                        <h3>{permission.label}</h3>
                        <p>
                          {permission.id === "accessibility" &&
                            "Needed to control windows and app focus."}
                          {permission.id === "screen-recording" &&
                            "Needed to capture screenshots during commands."}
                          {permission.id === "automation" &&
                            "Needed to run AppleScript automations."}
                        </p>
                      </div>
                      <div className="onboarding-permission__actions">
                        <span
                          className={`onboarding-status ${
                            permission.granted ? "is-good" : ""
                          }`}
                        >
                          {permission.granted ? "Granted" : "Not granted"}
                        </span>
                        <button
                          type="button"
                          onClick={() => handleOpenPermission(permission.id)}
                        >
                          Open Settings
                        </button>
                      </div>
                    </div>
                  ))
                )}
              </div>
              <div className="onboarding-note">
                Keep System Settings open until all permissions are enabled. We will
                automatically continue when everything is granted.
              </div>
            </div>
          )}

          {stepId === "finish" && (
            <div className="onboarding-card">
              <h3>Setup complete</h3>
              <p>
                Your workspace, AI provider, and permissions are ready. You can now
                summon the command bar and start automating.
              </p>
              <div className="onboarding-highlight">
                <div>
                  <h4>Try this first</h4>
                  <p>"Open Safari and search for latest macOS tips."</p>
                </div>
                <div>
                  <h4>Need tweaks?</h4>
                  <p>Open settings anytime with "/settings".</p>
                </div>
              </div>
            </div>
          )}

          {error && <div className="onboarding-error">{error}</div>}

          <div className="onboarding-actions">
            <button type="button" onClick={handleBack} disabled={stepIndex === 0 || busy}>
              Back
            </button>
            <button
              type="button"
              onClick={handleNext}
              disabled={
                busy ||
                (stepId === "permissions" && !allPermissionsGranted) ||
                (stepId === "workspace" && workspaceType === "remote")
              }
            >
              {busy ? "Working..." : stepId === "finish" ? "Enter Cocommand" : "Continue"}
            </button>
          </div>
        </section>
      </div>
    </main>
  );
}
