import { ReactNode, useCallback, useEffect, useRef, useState } from "react";
import { AppPanel, ButtonPrimary, Text } from "@cocommand/ui";
import { ApplicationProvider } from "../features/application/application.provider";
import { CommandProvider } from "../features/command/command.provider";
import { ExtensionProvider } from "../features/extension/extension.provider";
import { OnboardingProvider } from "../features/onboarding/onboarding.provider";
import { ServerProvider } from "../features/server/server.provider.tsx";
import { SessionProvider } from "../features/session/session.provider";
import { SettingsProvider } from "../features/settings/settings.provider";
import { WorkspaceProvider } from "../features/workspace/workspace.provider";
import { getServerInfo, ServerInfo } from "../lib/ipc.ts";

interface AppInitProps {
  children: ReactNode;
}

export function AppInit({ children }: AppInitProps) {
  const [serverInfo, setServerInfo] = useState<ServerInfo | null>(null);
  const fetchInFlight = useRef(false);

  const fetchServerInfo = useCallback(async (): Promise<void> => {
    if (fetchInFlight.current) return;
    fetchInFlight.current = true;
    try {
      const info = await getServerInfo();
      setServerInfo(info);
    } catch (error) {
      console.error("Failed to check server health:", error);
    } finally {
      fetchInFlight.current = false;
    }
  }, []);

  useEffect(() => {
    void fetchServerInfo();
  }, [fetchServerInfo]);

  useEffect(() => {
    if (serverInfo?.status === "ready" || serverInfo?.status === "error") return;
    const timer = window.setInterval(() => {
      void fetchServerInfo();
    }, 500);
    return () => window.clearInterval(timer);
  }, [serverInfo, fetchServerInfo]);

  if (!serverInfo || serverInfo.status !== "ready") {
    const isStarting = !serverInfo || serverInfo.status === "starting";
    return (
      <AppPanel style={{ minHeight: 360, maxWidth: 620 }}>
        <Text as="div" size="lg" weight="semibold">
          {isStarting ? "Starting Cocommand server..." : "Failed to start server"}
        </Text>
        <Text as="div" size="sm" tone="secondary">
          {isStarting ? "Waiting for backend startup." : serverInfo.error ?? "Unknown error."}
        </Text>
        {serverInfo?.status === "error" ? (
          <ButtonPrimary onClick={() => void fetchServerInfo()}>
            Retry
          </ButtonPrimary>
        ) : null}
      </AppPanel>
    );
  }

  return (
    <ServerProvider serverInfo={serverInfo}>
      <WorkspaceProvider>
        <OnboardingProvider>
          <ApplicationProvider>
            <SessionProvider>
              <CommandProvider>
                <SettingsProvider>
                  <ExtensionProvider>
                    {children}
                  </ExtensionProvider>
                </SettingsProvider>
              </CommandProvider>
            </SessionProvider>
          </ApplicationProvider>
        </OnboardingProvider>
      </WorkspaceProvider>
    </ServerProvider>
  );
}
