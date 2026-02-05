import "@cocommand/ui";
import { useEffect, useState } from "react";
import { useServerStore } from "../../state/server";
import { useAiStore } from "../../state/ai";
import { hideSettingsWindow } from "../../lib/ipc";
import styles from "./SettingsView.module.css";
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
    } catch (error) {
      setAiToast("error");
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
    <main className={styles.shell}>
      <AppPanel className={styles.panel}>
        <AppHeader
          title="Settings"
          subtitle="Configure your workspace and AI providers."
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
            <NavTab label="Overview" active={tab === "overview"} onClick={() => setTab("overview")} />
            <NavTab
              label="Workspace"
              active={tab === "workspace"}
              onClick={() => setTab("workspace")}
            />
            <NavTab label="AI" active={tab === "ai"} onClick={() => setTab("ai")} />
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
              <InlineHelp text="Workspace controls are coming next." />
            </InfoCard>
          )}

          {tab === "ai" && (
            <InfoCard>
              <Text as="h3" size="lg" weight="semibold">
                AI configuration
              </Text>
              <Text as="p" size="sm" tone="secondary">
                Configure the provider and model used by the command planner.
              </Text>
              <Divider />
              <Field label="Provider">
                <TextInput
                  value={aiForm.provider}
                  onChange={(event) => handleAiChange("provider", event.target.value)}
                  placeholder="openai-compatible"
                />
              </Field>
              <Field label="Base URL">
                <TextInput
                  value={aiForm.base_url}
                  onChange={(event) => handleAiChange("base_url", event.target.value)}
                  placeholder="https://api.openai.com/v1"
                />
              </Field>
              <Field label="Model">
                <TextInput
                  value={aiForm.model}
                  onChange={(event) => handleAiChange("model", event.target.value)}
                  placeholder="gpt-4o-mini"
                />
              </Field>
              <Field label="System prompt">
                <TextArea
                  value={aiForm.system_prompt}
                  onChange={(event) => handleAiChange("system_prompt", event.target.value)}
                  placeholder="You are Cocommand, a helpful command assistant."
                />
              </Field>
              <Field label="Temperature">
                <FieldRow>
                  <TextInput
                    value={aiForm.temperature}
                    onChange={(event) => handleAiChange("temperature", event.target.value)}
                    type="number"
                    step="0.1"
                    min="0"
                    max="2"
                  />
                  <Text size="xs" tone="tertiary">
                    Range 0–2
                  </Text>
                </FieldRow>
              </Field>
              <Field label="Max output tokens">
                <TextInput
                  value={aiForm.max_output_tokens}
                  onChange={(event) => handleAiChange("max_output_tokens", event.target.value)}
                  type="number"
                  min="256"
                />
              </Field>
              <Field label="Max steps">
                <TextInput
                  value={aiForm.max_steps}
                  onChange={(event) => handleAiChange("max_steps", event.target.value)}
                  type="number"
                  min="1"
                />
              </Field>
              <Field label="API key" help={aiSettings?.has_api_key ? "Stored securely" : undefined}>
                <TextInput
                  value={aiForm.api_key}
                  onChange={(event) => handleAiChange("api_key", event.target.value)}
                  placeholder={aiSettings?.has_api_key ? "Configured" : "sk-..."}
                  type="password"
                />
              </Field>
              <InlineHelp text="Required to continue unless already configured." />
              {serverInfo && aiError ? <ErrorBanner message={aiError} /> : null}
            </InfoCard>
          )}
        </AppContent>

        <AppFooter>
          <ButtonGroup>
            <ButtonSecondary onClick={hideSettingsWindow}>Close</ButtonSecondary>
            {tab === "ai" ? (
              <ButtonPrimary onClick={saveAiSettings} disabled={aiSaving}>
                {aiSaving ? "Saving..." : "Save changes"}
              </ButtonPrimary>
            ) : null}
          </ButtonGroup>
          {tab === "ai" ? (
            aiToast === "success" ? (
              <StatusBadge status="good" label="Settings saved" />
            ) : aiToast === "error" ? (
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
