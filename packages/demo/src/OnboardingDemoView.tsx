import "@cocommand/ui";
import { useState } from "react";
import styles from "./OnboardingDemoView.module.css";
import {
  AccentSwatch,
  AppContent,
  AppFooter,
  AppHeader,
  AppNav,
  AppPanel,
  AppShell,
  ButtonGroup,
  ButtonPrimary,
  ButtonSecondary,
  ChoiceCard,
  Divider,
  ErrorBanner,
  Field,
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

const accentOptions = [
  { id: "copper", label: "Copper", color: "#f46a4b" },
  { id: "sunset", label: "Sunset", color: "#f0a15c" },
  { id: "ember", label: "Ember", color: "#d25b7a" },
  { id: "sea", label: "Sea", color: "#3ea7a0" },
  { id: "electric", label: "Electric", color: "#4aa6ff" },
];

const steps = [
  "Welcome",
  "Workspace",
  "Extensions",
  "Theme",
  "AI",
  "Permissions",
  "Finish",
];

export function OnboardingDemoView() {
  const [watchRoot, setWatchRoot] = useState("~/");
  const [ignorePaths, setIgnorePaths] = useState(".git\nnode_modules\nLibrary/Caches");

  return (
    <AppShell className={`cc-theme-dark cc-reset ${styles.shell}`}>
      <AppPanel>
        <AppHeader
          title="Onboarding"
          subtitle="Set up your workspace, extensions, AI, and permissions in a few steps."
          brand={<img className={styles.brand} src="/logo_dark.png" alt="Cocommand" />}
          kicker={null}
        />

        <AppNav>
          <NavTabs>
            {steps.map((label, index) => (
              <NavTab
                key={label}
                label={label}
                active={label === "Extensions"}
                done={index < 2}
                leading={String(index + 1)}
              />
            ))}
          </NavTabs>
        </AppNav>

        <AppContent>
          <InfoCard>
            <Text as="h3" size="lg" weight="semibold">
              File system extension
            </Text>
            <Text as="p" size="sm" tone="secondary">
              Configure the default root and ignored folders used by filesystem tools.
            </Text>
            <div className={styles.permissionRow}>
              <div>
                <Text as="div" size="sm" weight="medium">
                  Extension status
                </Text>
                <Text as="div" size="sm" tone="secondary">
                  Built-in extension available in every workspace.
                </Text>
              </div>
              <StatusBadge status="good" label="Enabled" />
            </div>
            <Field label="Default watch root (watch_root)">
              <TextInput
                value={watchRoot}
                onChange={(event) => setWatchRoot(event.target.value)}
                placeholder="/"
              />
            </Field>
            <Field label="Ignore paths (ignore_paths)">
              <TextArea
                value={ignorePaths}
                onChange={(event) => setIgnorePaths(event.target.value)}
                placeholder={".git\nnode_modules\nLibrary/Caches"}
              />
              <InlineHelp text="Use one path per line. Supports absolute, ~/..., or watch-root relative paths." />
            </Field>
          </InfoCard>

          <InfoCard>
            <Text as="h3" size="lg" weight="semibold">
              Name your workspace
            </Text>
            <Text as="p" size="sm" tone="secondary">
              This name shows up in your command history and exports.
            </Text>

            <Field label="Workspace name">
              <TextInput placeholder="My workspace" />
            </Field>

            <Divider />

            <Field label="Theme mode">
              <OptionGroup className={styles.modeGrid}>
                <ChoiceCard title="System" description="Match macOS" selected />
                <ChoiceCard title="Light" description="Bright and clean" />
                <ChoiceCard title="Dark" description="Low light focus" />
              </OptionGroup>
            </Field>

            <Field label="Accent color">
              <div className={styles.accentRow}>
                {accentOptions.map((accent) => (
                  <AccentSwatch
                    key={accent.id}
                    label={accent.label}
                    color={accent.color}
                    selected={accent.id === "copper"}
                  />
                ))}
              </div>
            </Field>
          </InfoCard>

          <InfoCard>
            <Text as="h3" size="lg" weight="semibold">
              Permissions preview
            </Text>
            <Text as="p" size="sm" tone="secondary">
              Required permissions are shown below.
            </Text>
            <div className={styles.permissionRow}>
              <div>
                <Text as="div" size="sm" weight="medium">
                  Accessibility
                </Text>
                <Text as="div" size="sm" tone="secondary">
                  Needed to control windows and app focus.
                </Text>
              </div>
              <StatusBadge status="warn" label="Not granted" />
            </div>
            <div className={styles.permissionRow}>
              <div>
                <Text as="div" size="sm" weight="medium">
                  Screen recording
                </Text>
                <Text as="div" size="sm" tone="secondary">
                  Needed to capture screenshots during commands.
                </Text>
              </div>
              <StatusBadge status="good" label="Granted" />
            </div>
          </InfoCard>

          <InfoCard>
            <Text as="h3" size="lg" weight="semibold">
              AI settings preview
            </Text>
            <Field label="Provider">
              <TextInput placeholder="openai-compatible" />
            </Field>
            <Field label="Base URL">
              <TextInput placeholder="https://api.openai.com/v1" />
            </Field>
            <Field label="System prompt">
              <TextArea placeholder="You are Cocommand, a helpful command assistant." />
            </Field>
            <InlineHelp text="API key required if not already configured." />
          </InfoCard>

          <InfoCard>
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

          <ErrorBanner message="Sample error message would appear here." />
        </AppContent>

        <AppFooter>
          <ButtonGroup>
            <ButtonSecondary>Back</ButtonSecondary>
            <ButtonPrimary>Continue</ButtonPrimary>
          </ButtonGroup>
          <Text size="xs" tone="tertiary">
            Step 3 of 7
          </Text>
        </AppFooter>
      </AppPanel>
    </AppShell>
  );
}
