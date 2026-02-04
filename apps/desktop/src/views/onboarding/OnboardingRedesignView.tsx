import "@cocommand/ui";
import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useServerStore } from "../../state/server";
import { useOnboardingStore } from "../../state/onboarding";
import { useWorkspaceStore } from "../../state/workspace";
import { useAiStore } from "../../state/ai";
import { usePermissionsStore } from "../../state/permissions";
import { setWorkspaceDir } from "../../lib/ipc";
import styles from "./OnboardingRedesignView.module.css";
import {
  AccentSwatch,
  AppContent,
  AppFooter,
  AppHeader,
  AppNav,
  AppPanel,
  ButtonGroup,
  ButtonPrimary,
  ButtonSecondary,
  ChoiceCard,
  ErrorBanner,
  Field,
  FieldRow,
  HighlightGrid,
  HighlightItem,
  InfoCard,
  InlineHelp,
  NavTab,
  NavTabs,
  OptionGroup,
  StatusBadge,
  Text,
  TextArea,
  TextInput,
} from "@cocommand/ui";

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

export function OnboardingRedesignView() {
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
    <AppPanel className={styles.panel}>
      <AppHeader
        title="Onboarding"
        subtitle={steps[stepIndex]?.subtitle}
        brand={<img className={styles.brand} src="/logo_dark.png" alt="Cocommand" />}
        kicker={null}
      />

      <AppNav>
        <NavTabs>
          {steps.map((step, index) => (
            <NavTab
              key={step.id}
              label={step.title}
              active={index === stepIndex}
              done={index < stepIndex}
              leading={String(index + 1)}
            />
          ))}
        </NavTabs>
      </AppNav>

      <AppContent>
        {stepId === "welcome" && (
          <InfoCard>
            <Text as="h3" size="lg" weight="semibold">
              Let's set up your command center
            </Text>
            <Text as="p" size="sm" tone="secondary">
              Cocommand uses a workspace to store sessions, models, and automation
              preferences. We'll also enable macOS permissions so commands run without
              interruptions.
            </Text>
            <HighlightGrid>
              <HighlightItem
                title="Fast setup"
                description="Most teams finish in under two minutes."
              />
              <HighlightItem
                title="Secure by default"
                description="Your keys and settings stay on this device."
              />
            </HighlightGrid>
          </InfoCard>
        )}

        {stepId === "workspace" && (
          <InfoCard>
            <Field label="Workspace type">
              <OptionGroup className={styles.modeGrid}>
                <ChoiceCard
                  title="Local workspace"
                  description="Store data on this Mac for maximum speed."
                  selected={workspaceType === "local"}
                  onClick={() => setWorkspaceType("local")}
                />
                <ChoiceCard
                  title="Remote workspace"
                  description="Connect to a server in the cloud. Coming soon."
                  disabled
                />
              </OptionGroup>
            </Field>
            <Field label="Workspace folder">
              <FieldRow>
                <TextInput value={workspacePath} readOnly />
                <ButtonSecondary onClick={handlePickWorkspace}>Choose</ButtonSecondary>
              </FieldRow>
              <InlineHelp text="This folder will store sessions, history, and extensions." />
            </Field>
          </InfoCard>
        )}

        {stepId === "theme" && (
          <InfoCard>
            <Field label="Workspace name">
              <TextInput
                value={workspaceName}
                onChange={(event) => setWorkspaceName(event.target.value)}
                placeholder="My workspace"
              />
            </Field>
            <Field label="Theme mode">
              <OptionGroup className={styles.modeGrid}>
                {themeModes.map((mode) => (
                  <ChoiceCard
                    key={mode.id}
                    title={mode.label}
                    selected={themeMode === mode.id}
                    onClick={() => setThemeMode(mode.id)}
                  />
                ))}
              </OptionGroup>
            </Field>
            <Field label="Accent color">
              <div className={styles.accentRow}>
                {accentOptions.map((accent) => (
                  <AccentSwatch
                    key={accent.id}
                    label={accent.label}
                    color={accent.color}
                    selected={themeAccent === accent.id}
                    onClick={() => setThemeAccent(accent.id)}
                  />
                ))}
              </div>
            </Field>
          </InfoCard>
        )}

        {stepId === "ai" && (
          <InfoCard>
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
            <Field label="API key">
              <TextInput
                type="password"
                value={aiForm.api_key}
                onChange={(event) => handleAiChange("api_key", event.target.value)}
                placeholder={aiSettings?.has_api_key ? "Configured" : "sk-..."}
              />
              <InlineHelp text="Required to continue unless already configured." />
            </Field>
          </InfoCard>
        )}

        {stepId === "permissions" && (
          <InfoCard>
            <Text as="h3" size="lg" weight="semibold">
              Required permissions
            </Text>
            <Text as="p" size="sm" tone="secondary">
              Keep System Settings open until all permissions are enabled.
            </Text>
            <div className={styles.permissionsList}>
              {permissions.length === 0 ? (
                <div className={styles.permissionRow}>Checking permissions...</div>
              ) : (
                permissions.map((permission) => (
                  <div key={permission.id} className={styles.permissionRow}>
                    <div>
                      <Text as="div" size="sm" weight="medium">
                        {permission.label}
                      </Text>
                      <Text as="div" size="sm" tone="secondary">
                        {permission.id === "accessibility" &&
                          "Needed to control windows and app focus."}
                        {permission.id === "screen-recording" &&
                          "Needed to capture screenshots during commands."}
                        {permission.id === "automation" &&
                          "Needed to run AppleScript automations."}
                      </Text>
                    </div>
                    <div className={styles.permissionActions}>
                      <StatusBadge
                        status={permission.granted ? "good" : "warn"}
                        label={permission.granted ? "Granted" : "Not granted"}
                      />
                      <ButtonSecondary onClick={() => handleOpenPermission(permission.id)}>
                        Open Settings
                      </ButtonSecondary>
                    </div>
                  </div>
                ))
              )}
            </div>
          </InfoCard>
        )}

        {stepId === "finish" && (
          <InfoCard>
            <Text as="h3" size="lg" weight="semibold">
              Setup complete
            </Text>
            <Text as="p" size="sm" tone="secondary">
              Your workspace, AI provider, and permissions are ready. You can now
              summon the command bar and start automating.
            </Text>
            <HighlightGrid>
              <HighlightItem
                title="Try this first"
                description="Open Safari and search for latest macOS tips."
              />
              <HighlightItem
                title="Need tweaks?"
                description="Open settings anytime with /settings."
              />
            </HighlightGrid>
          </InfoCard>
        )}

        {error ? <ErrorBanner message={error} /> : null}
      </AppContent>

      <AppFooter>
        <ButtonGroup>
          <ButtonSecondary onClick={handleBack} disabled={stepIndex === 0 || busy}>
            Back
          </ButtonSecondary>
          <ButtonPrimary
            onClick={handleNext}
            disabled={
              busy ||
              (stepId === "permissions" && !allPermissionsGranted) ||
              (stepId === "workspace" && workspaceType === "remote")
            }
          >
            {busy ? "Working..." : stepId === "finish" ? "Enter Cocommand" : "Continue"}
          </ButtonPrimary>
        </ButtonGroup>
        <Text size="xs" tone="tertiary">
          Step {stepIndex + 1} of {steps.length}
        </Text>
      </AppFooter>
    </AppPanel>
  );
}
