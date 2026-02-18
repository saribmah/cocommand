import "@cocommand/ui";
import { useEffect, useState } from "react";
import { useStore } from "zustand";
import { useExtensionStore } from "../extension/extension.context";
import type { WorkspaceExtensionState } from "./workspace.extension-store";
import type { ExtensionViewProps } from "../extension/extension-views";
import styles from "./workspace.module.css";
import {
  AccentSwatch,
  ButtonPrimary,
  ButtonSecondary,
  ChoiceCard,
  Divider,
  ErrorBanner,
  Field,
  FieldRow,
  InlineHelp,
  OptionGroup,
  StatusBadge,
  Text,
  TextArea,
  TextInput,
} from "@cocommand/ui";

type SettingsTab = "general" | "ai" | "permissions";

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

const tabs: { id: SettingsTab; label: string }[] = [
  { id: "general", label: "General" },
  { id: "ai", label: "AI" },
  { id: "permissions", label: "Permissions" },
];

export function WorkspaceView({ mode }: ExtensionViewProps) {
  const store = useExtensionStore<WorkspaceExtensionState>("workspace");
  const config = useStore(store, (s) => s.config);
  const isLoading = useStore(store, (s) => s.isLoading);
  const isSaving = useStore(store, (s) => s.isSaving);
  const storeError = useStore(store, (s) => s.error);
  const permissions = useStore(store, (s) => s.permissions);
  const fetchConfig = useStore(store, (s) => s.fetchConfig);
  const updateConfig = useStore(store, (s) => s.updateConfig);
  const fetchPermissions = useStore(store, (s) => s.fetchPermissions);
  const openPermission = useStore(store, (s) => s.openPermission);

  const [tab, setTab] = useState<SettingsTab>("general");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [toast, setToast] = useState<"success" | "error" | null>(null);

  // General tab state
  const [workspaceName, setWorkspaceName] = useState("");
  const [themeMode, setThemeMode] = useState("system");
  const [themeAccent, setThemeAccent] = useState("copper");

  // AI tab state
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

  const isSetup = config ? !config.onboarding.completed : false;
  const isInline = mode === "inline";

  // Fetch config on mount
  useEffect(() => {
    if (!config && !isLoading) void fetchConfig();
  }, [config, isLoading, fetchConfig]);

  // Sync form state from config
  useEffect(() => {
    if (!config) return;
    setWorkspaceName(config.name);
    setThemeMode(config.theme.mode);
    setThemeAccent(config.theme.accent);
  }, [config]);

  useEffect(() => {
    if (!config) return;
    setAiForm({
      provider: config.llm.provider,
      base_url: config.llm.base_url,
      model: config.llm.model,
      system_prompt: config.llm.system_prompt,
      temperature: String(config.llm.temperature ?? 0.7),
      max_output_tokens: String(config.llm.max_output_tokens ?? 80000),
      max_steps: String(config.llm.max_steps ?? 8),
      api_key: "",
    });
  }, [config]);

  // Fetch permissions on mount and poll on permissions tab
  useEffect(() => {
    if (!permissions.length) void fetchPermissions();
  }, [permissions.length, fetchPermissions]);

  useEffect(() => {
    if (tab !== "permissions") return;
    const timer = window.setInterval(() => {
      void fetchPermissions();
    }, 4500);
    return () => window.clearInterval(timer);
  }, [tab, fetchPermissions]);

  // Toast auto-dismiss
  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), 2500);
    return () => window.clearTimeout(timer);
  }, [toast]);

  const handleAiChange = (field: keyof typeof aiForm, value: string) => {
    setAiForm((prev) => ({ ...prev, [field]: value }));
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

  const handleSave = async () => {
    if (!config || busy) return;
    setError(null);
    setBusy(true);
    setToast(null);

    try {
      const apiKeyInput = aiForm.api_key.trim();
      const updated = {
        ...config,
        name: workspaceName.trim() || config.name,
        theme: {
          ...config.theme,
          mode: themeMode,
          accent: themeAccent,
        },
        llm: {
          ...config.llm,
          provider: aiForm.provider.trim(),
          base_url: aiForm.base_url.trim(),
          model: aiForm.model.trim(),
          system_prompt: aiForm.system_prompt.trim(),
          temperature: Number(aiForm.temperature),
          max_output_tokens: Number(aiForm.max_output_tokens),
          max_steps: Number(aiForm.max_steps),
          api_key: apiKeyInput.length > 0 ? apiKeyInput : config.llm.api_key,
        },
        ...(isSetup
          ? {
              onboarding: {
                ...config.onboarding,
                completed: true,
                completed_at: Math.floor(Date.now() / 1000),
              },
            }
          : {}),
      };

      await updateConfig(updated);
      setAiForm((prev) => ({ ...prev, api_key: "" }));
      setToast("success");
    } catch (err) {
      setError(String(err));
      setToast("error");
    } finally {
      setBusy(false);
    }
  };

  if (!config) {
    return (
      <div className={`${isInline ? "" : "app-shell "}${styles.container}${isInline ? ` ${styles.inline}` : ""}`}>
        <div className={styles.loading}>
          <Text as="div" size="lg" weight="semibold">
            {storeError ? "Failed to load workspace config" : "Loading workspace..."}
          </Text>
          {storeError ? (
            <Text as="div" size="sm" tone="secondary">{storeError}</Text>
          ) : null}
        </div>
      </div>
    );
  }

  return (
    <div className={`${isInline ? "" : "app-shell "}${styles.container}${isInline ? ` ${styles.inline}` : ""}`}>
      {/* Sidebar */}
      <div className={styles.sidebar}>
        <div className={styles.sidebarHeader}>
          <Text as="div" size="md" weight="semibold">
            {isSetup ? "Setup" : "Settings"}
          </Text>
        </div>
        <div className={styles.tabList}>
          {tabs.map((t) => (
            <button
              key={t.id}
              type="button"
              className={`${styles.tabItem}${tab === t.id ? ` ${styles.tabItemSelected}` : ""}`}
              onClick={() => setTab(t.id)}
            >
              {t.label}
            </button>
          ))}
        </div>
      </div>

      {/* Detail panel */}
      <div className={styles.detail}>
        <div className={styles.detailContent}>
          {tab === "general" && (
            <>
              <Text as="h3" size="lg" weight="semibold">
                General
              </Text>
              <Divider />
              <Field label="Workspace name">
                <TextInput
                  value={workspaceName}
                  onChange={(e) => setWorkspaceName(e.target.value)}
                  placeholder="My workspace"
                />
              </Field>
              <Field label="Theme mode">
                <OptionGroup className={styles.modeGrid}>
                  {themeModes.map((m) => (
                    <ChoiceCard
                      key={m.id}
                      title={m.label}
                      selected={themeMode === m.id}
                      onClick={() => setThemeMode(m.id)}
                    />
                  ))}
                </OptionGroup>
              </Field>
              <Field label="Accent color">
                <div className={styles.accentRow}>
                  {accentOptions.map((a) => (
                    <AccentSwatch
                      key={a.id}
                      label={a.label}
                      color={a.color}
                      selected={themeAccent === a.id}
                      onClick={() => setThemeAccent(a.id)}
                    />
                  ))}
                </div>
              </Field>
            </>
          )}

          {tab === "ai" && (
            <>
              <Text as="h3" size="lg" weight="semibold">
                AI configuration
              </Text>
              <Divider />
              <Field label="Provider">
                <TextInput
                  value={aiForm.provider}
                  onChange={(e) => handleAiChange("provider", e.target.value)}
                  placeholder="openai-compatible"
                />
              </Field>
              <Field label="Base URL">
                <TextInput
                  value={aiForm.base_url}
                  onChange={(e) => handleAiChange("base_url", e.target.value)}
                  placeholder="https://api.openai.com/v1"
                />
              </Field>
              <Field label="Model">
                <TextInput
                  value={aiForm.model}
                  onChange={(e) => handleAiChange("model", e.target.value)}
                  placeholder="gpt-4o-mini"
                />
              </Field>
              <Field label="System prompt">
                <TextArea
                  value={aiForm.system_prompt}
                  onChange={(e) => handleAiChange("system_prompt", e.target.value)}
                  placeholder="You are Cocommand, a helpful command assistant."
                />
              </Field>
              <Field label="Temperature">
                <FieldRow>
                  <TextInput
                    value={aiForm.temperature}
                    onChange={(e) => handleAiChange("temperature", e.target.value)}
                    type="number"
                    step="0.1"
                    min="0"
                    max="2"
                  />
                  <Text size="xs" tone="tertiary">
                    Range 0-2
                  </Text>
                </FieldRow>
              </Field>
              <Field label="Max output tokens">
                <TextInput
                  value={aiForm.max_output_tokens}
                  onChange={(e) => handleAiChange("max_output_tokens", e.target.value)}
                  type="number"
                  min="256"
                />
              </Field>
              <Field label="Max steps">
                <TextInput
                  value={aiForm.max_steps}
                  onChange={(e) => handleAiChange("max_steps", e.target.value)}
                  type="number"
                  min="1"
                />
              </Field>
              <Field label="API key">
                <TextInput
                  type="password"
                  value={aiForm.api_key}
                  onChange={(e) => handleAiChange("api_key", e.target.value)}
                  placeholder={
                    (config.llm.api_key ?? "").trim().length > 0
                      ? "Configured"
                      : "sk-..."
                  }
                />
                <InlineHelp text="Leave blank to keep the current key." />
              </Field>
            </>
          )}

          {tab === "permissions" && (
            <>
              <Text as="h3" size="lg" weight="semibold">
                Permissions
              </Text>
              <Text as="p" size="sm" tone="secondary">
                macOS permissions required for automation.
              </Text>
              <Divider />
              <div className={styles.permissionsList}>
                {permissions.length === 0 ? (
                  <div className={styles.permissionRow}>Checking permissions...</div>
                ) : (
                  permissions.map((p) => (
                    <div key={p.id} className={styles.permissionRow}>
                      <div>
                        <Text as="div" size="sm" weight="medium">
                          {p.label}
                        </Text>
                        <Text as="div" size="sm" tone="secondary">
                          {p.id === "accessibility" &&
                            "Needed to control windows and app focus."}
                          {p.id === "screen-recording" &&
                            "Needed to capture screenshots during commands."}
                          {p.id === "automation" &&
                            "Needed to run AppleScript automations."}
                        </Text>
                      </div>
                      <div className={styles.permissionActions}>
                        <StatusBadge
                          status={p.granted ? "good" : "warn"}
                          label={p.granted ? "Granted" : "Not granted"}
                        />
                        <ButtonSecondary onClick={() => handleOpenPermission(p.id)}>
                          Open Settings
                        </ButtonSecondary>
                      </div>
                    </div>
                  ))
                )}
              </div>
            </>
          )}

          {error ? <ErrorBanner message={error} /> : null}
        </div>

        <div className={styles.detailFooter}>
          <ButtonPrimary onClick={handleSave} disabled={busy || isSaving}>
            {busy || isSaving
              ? "Saving..."
              : isSetup
                ? "Complete Setup"
                : "Save"}
          </ButtonPrimary>
          {toast === "success" ? (
            <StatusBadge status="good" label={isSetup ? "Setup complete" : "Settings saved"} />
          ) : toast === "error" ? (
            <StatusBadge status="warn" label="Save failed" />
          ) : (
            <Text size="xs" tone="tertiary">
              {isSetup
                ? "Configure settings and complete setup"
                : mode === "popout"
                  ? "Press Esc to close"
                  : ""}
            </Text>
          )}
        </div>
      </div>
    </div>
  );
}
