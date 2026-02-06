import "@cocommand/ui";
import styles from "./SettingsDemoView.module.css";
import {
  AppContent,
  AppFooter,
  AppHeader,
  AppNav,
  AppPanel,
  AppShell,
  ButtonGroup,
  ButtonPrimary,
  ButtonSecondary,
  Divider,
  Field,
  FieldRow,
  InfoCard,
  NavTab,
  NavTabs,
  StatusBadge,
  Text,
  TextArea,
  TextInput,
} from "@cocommand/ui";

const tabs = ["Overview", "Workspace", "AI"];

export function SettingsDemoView() {
  return (
    <AppShell className={`cc-theme-dark cc-reset ${styles.shell}`}>
      <AppPanel>
        <AppHeader
          title="Settings"
          subtitle="Configure your workspace and AI providers."
          brand={<img className={styles.brand} src="/logo_dark.png" alt="Cocommand" />}
          kicker={null}
          meta={
            <div className={styles.status}>
              <StatusBadge status="good" label="Server running" />
              <Text size="xs" tone="tertiary">
                127.0.0.1:4840
              </Text>
            </div>
          }
        />

        <AppNav>
          <NavTabs>
            {tabs.map((label) => (
              <NavTab key={label} label={label} active={label === "AI"} />
            ))}
          </NavTabs>
        </AppNav>

        <AppContent>
          <InfoCard>
            <Text as="h3" size="lg" weight="semibold">
              AI Configuration
            </Text>
            <Text as="p" size="sm" tone="secondary">
              Configure the provider and model used by the command planner.
            </Text>
            <Divider />
            <Field label="Provider">
              <TextInput placeholder="openai-compatible" />
            </Field>
            <Field label="Base URL">
              <TextInput placeholder="https://api.openai.com/v1" />
            </Field>
            <Field label="Model">
              <TextInput placeholder="gpt-4o-mini" />
            </Field>
            <Field label="System prompt">
              <TextArea placeholder="You are Cocommand, a helpful command assistant." />
            </Field>
            <Field label="Temperature">
              <FieldRow>
                <TextInput type="number" min="0" max="2" step="0.1" defaultValue="0.7" />
                <Text size="xs" tone="tertiary">
                  Range 0–2
                </Text>
              </FieldRow>
            </Field>
            <Field label="API key">
              <TextInput type="password" placeholder="sk-..." />
            </Field>
          </InfoCard>
        </AppContent>

        <AppFooter>
          <ButtonGroup>
            <ButtonSecondary>Cancel</ButtonSecondary>
            <ButtonPrimary>Save changes</ButtonPrimary>
          </ButtonGroup>
          <Text size="xs" tone="tertiary">
            Last saved 2 minutes ago
          </Text>
        </AppFooter>
      </AppPanel>
    </AppShell>
  );
}
