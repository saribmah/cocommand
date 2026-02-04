import "@cocommand/ui";
import styles from "./ResponsesDemoView.module.css";
import {
  ActionRow,
  AppContent,
  AppFooter,
  AppHeader,
  AppNav,
  AppPanel,
  AppShell,
  ButtonGroup,
  ButtonPrimary,
  ButtonSecondary,
  FileCard,
  NavTab,
  NavTabs,
  ReasoningCard,
  ResponseMeta,
  ResponseStack,
  StatusBadge,
  Text,
  TextResponseCard,
  ToolCallCard,
} from "@cocommand/ui";

const tabs = ["Overview", "Responses", "Tools"];

export function ResponsesDemoView() {
  return (
    <AppShell className={`cc-theme-dark cc-reset ${styles.shell}`}>
      <AppPanel>
        <AppHeader
          title="Responses"
          subtitle="Preview how LLM responses and tool calls are presented."
          brand={<img className={styles.brand} src="/logo_dark.png" alt="Cocommand" />}
          kicker={null}
          meta={
            <div className={styles.status}>
              <StatusBadge status="good" label="Live session" />
              <Text size="xs" tone="tertiary">
                2 tools pending
              </Text>
            </div>
          }
        />

        <AppNav>
          <NavTabs>
            {tabs.map((label) => (
              <NavTab key={label} label={label} active={label === "Responses"} />
            ))}
          </NavTabs>
        </AppNav>

        <AppContent>
          <ResponseStack>
            <TextResponseCard
              body={`Sure — I can do that.\n\nHere's a quick summary of what I'll do:\n- Search your workspace\n- Run the tool\n- Return the report`}
            />

            <ToolCallCard
              toolName="search_workspace"
              toolId="tool_182"
              state="running"
              params={`{"query":"quarterly revenue","limit":5}`}
            />

            <ToolCallCard
              toolName="generate_report"
              toolId="tool_183"
              state="success"
              params={`{"format":"pdf","sections":["summary","tables"]}`}
              result={`{"file":"report-q3.pdf","pages":12}`}
            />

            <FileCard
              fileName="report-q3.pdf"
              fileType="PDF"
              fileSize="1.8 MB"
              actions={
                <ActionRow>
                  <ButtonSecondary>Reveal</ButtonSecondary>
                  <ButtonPrimary>Open</ButtonPrimary>
                </ActionRow>
              }
            />

            <ReasoningCard
              reasoning={`Plan:\n1. Locate the revenue data.\n2. Summarize the key changes.\n3. Export to PDF.`}
            />

            <TextResponseCard
              label="Assistant"
              body={`Done! The report is ready. I also flagged two anomalies in Q3.`}
            />
          </ResponseStack>
        </AppContent>

        <AppFooter>
          <ButtonGroup>
            <ButtonSecondary>Clear</ButtonSecondary>
            <ButtonPrimary>Send follow-up</ButtonPrimary>
          </ButtonGroup>
          <ResponseMeta items={["512 tokens", "3.2s", "Model: gpt-4o-mini"]} />
        </AppFooter>
      </AppPanel>
    </AppShell>
  );
}
