import "@cocommand/ui";
import { useEffect, useState } from "react";
import { hideSettingsWindow } from "../../lib/ipc";
import { useServerContext } from "../server/server.context";
import { useWorkspaceContext } from "../workspace/workspace.context";
import styles from "./settings.module.css";
import {
  AppContent,
  AppFooter,
  AppHeader,
  AppNav,
  AppPanel,
  ButtonGroup,
  ButtonPrimary,
  ButtonSecondary,
  Divider,
  ErrorBanner,
  Field,
  FieldRow,
  InfoCard,
  InlineHelp,
  NavTab,
  NavTabs,
  Pill,
  StatusBadge,
  Text,
  TextArea,
  TextInput,
} from "@cocommand/ui";

export function SettingsView() {
  const serverInfo = useServerContext((state) => state.info);
  const workspaceConfig = useWorkspaceContext((state) => state.config);
  const workspaceLoaded = useWorkspaceContext((state) => state.isLoaded);
  const workspaceError = useWorkspaceContext((state) => state.error);
  const fetchWorkspaceConfig = useWorkspaceContext((state) => state.fetchConfig);
  const updateWorkspaceConfig = useWorkspaceContext((state) => state.updateConfig);

  const [tab, setTab] = useState<"overview" | "workspace" | "llm">("overview");
  const [llmForm, setLlmForm] = useState({
    provider: "openai-compatible",
    base_url: "",
    model: "",
    system_prompt: "",
    temperature: "0.7",
    max_output_tokens: "80000",
    max_steps: "8",
    api_key: "",
  });
  const [llmSaving, setLlmSaving] = useState(false);
  const [llmToast, setLlmToast] = useState<null | "success" | "error">(null);

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
    if (!workspaceLoaded) {
      void fetchWorkspaceConfig();
    }
  }, [serverInfo, workspaceLoaded, fetchWorkspaceConfig]);

  useEffect(() => {
    if (!workspaceConfig) return;
    setLlmForm({
      provider: workspaceConfig.llm.provider,
      base_url: workspaceConfig.llm.base_url,
      model: workspaceConfig.llm.model,
      system_prompt: workspaceConfig.llm.system_prompt,
      temperature: String(workspaceConfig.llm.temperature ?? 0.7),
      max_output_tokens: String(workspaceConfig.llm.max_output_tokens ?? 80000),
      max_steps: String(workspaceConfig.llm.max_steps ?? 8),
      api_key: "",
    });
  }, [workspaceConfig]);

  const handleLlmChange = (field: keyof typeof llmForm, value: string) => {
    setLlmForm((prev) => ({ ...prev, [field]: value }));
  };

  const saveLlmSettings = async () => {
    if (!workspaceConfig) {
      setLlmToast("error");
      return;
    }

    setLlmSaving(true);
    setLlmToast(null);
    try {
      const apiKeyInput = llmForm.api_key.trim();
      await updateWorkspaceConfig({
        ...workspaceConfig,
        llm: {
          ...workspaceConfig.llm,
          provider: llmForm.provider.trim(),
          base_url: llmForm.base_url.trim(),
          model: llmForm.model.trim(),
          system_prompt: llmForm.system_prompt.trim(),
          temperature: Number(llmForm.temperature),
          max_output_tokens: Number(llmForm.max_output_tokens),
          max_steps: Number(llmForm.max_steps),
          api_key: apiKeyInput.length > 0 ? apiKeyInput : workspaceConfig.llm.api_key,
        },
      });
      setLlmForm((prev) => ({ ...prev, api_key: "" }));
      setLlmToast("success");
    } catch {
      setLlmToast("error");
    } finally {
      setLlmSaving(false);
    }
  };

  useEffect(() => {
    if (!llmToast) return;
    const timer = window.setTimeout(() => setLlmToast(null), 2500);
    return () => window.clearTimeout(timer);
  }, [llmToast]);

  return (
    <main className={styles.shell}>
      <AppPanel className={styles.panel}>
        <AppHeader
          title="Settings"
          subtitle="Configure your workspace and model providers."
          brand={<img className={styles.brand} src="/logo_dark.png" alt="Cocommand" />}
          kicker={null}
          meta={
            <div className={styles.status}>
              <StatusBadge
                status={serverInfo ? "good" : "warn"}
                label={serverInfo ? "Server running" : "Server offline"}
              />
              <Text size="xs" tone="tertiary">
                {serverInfo?.addr ?? "Not connected"}
              </Text>
            </div>
          }
        />

        <AppNav>
          <NavTabs>
            <NavTab
              label="Overview"
              active={tab === "overview"}
              onClick={() => setTab("overview")}
            />
            <NavTab
              label="Workspace"
              active={tab === "workspace"}
              onClick={() => setTab("workspace")}
            />
            <NavTab label="LLM" active={tab === "llm"} onClick={() => setTab("llm")} />
          </NavTabs>
        </AppNav>

        <AppContent className={styles.content}>
          {tab === "overview" && (
            <InfoCard>
              <Text as="h3" size="lg" weight="semibold">
                Overview
              </Text>
              <Text as="p" size="sm" tone="secondary">
                Core configuration for this device.
              </Text>
              <Divider />
              <Field label="Server address">
                <Text size="sm">{serverInfo?.addr ?? "Not connected"}</Text>
              </Field>
              <Field label="Workspace path">
                <Text size="sm">{serverInfo?.workspace_dir ?? "Unknown"}</Text>
              </Field>
              <Field label="Shortcuts">
                <div className={styles.shortcutRow}>
                  <Pill>/settings</Pill>
                  <Pill>Esc</Pill>
                  <Pill>/help</Pill>
                </div>
              </Field>
            </InfoCard>
          )}

          {tab === "workspace" && (
            <InfoCard>
              <Text as="h3" size="lg" weight="semibold">
                Workspace
              </Text>
              <Text as="p" size="sm" tone="secondary">
                Configure where Cocommand stores sessions and files.
              </Text>
              <Divider />
              <Field label="Workspace path">
                <TextInput value={serverInfo?.workspace_dir ?? ""} readOnly />
              </Field>
              <Field label="Workspace name">
                <TextInput value={workspaceConfig?.name ?? ""} readOnly />
              </Field>
              <InlineHelp text="Workspace controls are coming next." />
            </InfoCard>
          )}

          {tab === "llm" && (
            <InfoCard>
              <Text as="h3" size="lg" weight="semibold">
                LLM configuration
              </Text>
              <Text as="p" size="sm" tone="secondary">
                Configure the provider and model used by the command planner.
              </Text>
              <Divider />
              <Field label="Provider">
                <TextInput
                  value={llmForm.provider}
                  onChange={(event) => handleLlmChange("provider", event.target.value)}
                  placeholder="openai-compatible"
                />
              </Field>
              <Field label="Base URL">
                <TextInput
                  value={llmForm.base_url}
                  onChange={(event) => handleLlmChange("base_url", event.target.value)}
                  placeholder="https://api.openai.com/v1"
                />
              </Field>
              <Field label="Model">
                <TextInput
                  value={llmForm.model}
                  onChange={(event) => handleLlmChange("model", event.target.value)}
                  placeholder="gpt-4o-mini"
                />
              </Field>
              <Field label="System prompt">
                <TextArea
                  value={llmForm.system_prompt}
                  onChange={(event) => handleLlmChange("system_prompt", event.target.value)}
                  placeholder="You are Cocommand, a helpful command assistant."
                />
              </Field>
              <Field label="Temperature">
                <FieldRow>
                  <TextInput
                    value={llmForm.temperature}
                    onChange={(event) => handleLlmChange("temperature", event.target.value)}
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
                  value={llmForm.max_output_tokens}
                  onChange={(event) => handleLlmChange("max_output_tokens", event.target.value)}
                  type="number"
                  min="256"
                />
              </Field>
              <Field label="Max steps">
                <TextInput
                  value={llmForm.max_steps}
                  onChange={(event) => handleLlmChange("max_steps", event.target.value)}
                  type="number"
                  min="1"
                />
              </Field>
              <Field
                label="API key"
                help={
                  (workspaceConfig?.llm.api_key ?? "").trim().length > 0
                    ? "Stored securely"
                    : undefined
                }
              >
                <TextInput
                  value={llmForm.api_key}
                  onChange={(event) => handleLlmChange("api_key", event.target.value)}
                  placeholder={
                    (workspaceConfig?.llm.api_key ?? "").trim().length > 0
                      ? "Configured"
                      : "sk-..."
                  }
                  type="password"
                />
              </Field>
              <InlineHelp text="Required to continue unless already configured." />
              {serverInfo && workspaceError ? <ErrorBanner message={workspaceError} /> : null}
            </InfoCard>
          )}
        </AppContent>

        <AppFooter>
          <ButtonGroup>
            <ButtonSecondary onClick={hideSettingsWindow}>Close</ButtonSecondary>
            {tab === "llm" ? (
              <ButtonPrimary onClick={saveLlmSettings} disabled={llmSaving}>
                {llmSaving ? "Saving..." : "Save changes"}
              </ButtonPrimary>
            ) : null}
          </ButtonGroup>
          {tab === "llm" ? (
            llmToast === "success" ? (
              <StatusBadge status="good" label="Settings saved" />
            ) : llmToast === "error" ? (
              <StatusBadge status="warn" label="Save failed" />
            ) : (
              <Text size="xs" tone="tertiary">
                Press Esc to close
              </Text>
            )
          ) : (
            <Text size="xs" tone="tertiary">
              Press Esc to close
            </Text>
          )}
        </AppFooter>
      </AppPanel>
    </main>
  );
}
