import type { ReactNode } from "react";
import { useEffect } from "react";
import { useServerStore } from "../state/server";
import { AppPanel, ButtonPrimary, Text } from "@cocommand/ui";
import { ServerContext } from "../context/ServerContext";

interface AppInitProps {
  children: ReactNode;
}

export function AppInit({ children }: AppInitProps) {
  const fetchServerInfo = useServerStore((state) => state.fetchInfo);
  const fetchServerStatus = useServerStore((state) => state.fetchStatus);
  const serverStatus = useServerStore((state) => state.status);
  const serverStatusError = useServerStore((state) => state.statusError);
  const serverInfo = useServerStore((state) => state.info);
  const workspaceDir = useServerStore((state) => state.workspaceDir);

  useEffect(() => {
    fetchServerStatus();
  }, [fetchServerStatus]);

  useEffect(() => {
    if (serverStatus !== "ready") return;
    fetchServerInfo();
  }, [serverStatus, fetchServerInfo]);

  useEffect(() => {
    if (serverStatus === "ready" || serverStatus === "error") return;
    const timer = window.setInterval(() => {
      fetchServerStatus();
    }, 500);
    return () => window.clearInterval(timer);
  }, [serverStatus, fetchServerStatus]);

  if (serverStatus !== "ready") {
    return (
      <ServerContext.Provider
        value={{
          status: serverStatus,
          statusError: serverStatusError,
          info: serverInfo,
          workspaceDir,
          refreshStatus: fetchServerStatus,
        }}
      >
        <AppPanel style={{ minHeight: 360, maxWidth: 620 }}>
          <Text as="div" size="lg" weight="semibold">
            {serverStatus === "starting"
              ? "Starting Cocommand server..."
              : "Failed to start server"}
          </Text>
          <Text as="div" size="sm" tone="secondary">
            {serverStatus === "starting"
              ? "Waiting for backend startup."
              : serverStatusError ?? "Unknown error."}
          </Text>
          {serverStatus === "error" ? (
            <ButtonPrimary onClick={() => fetchServerStatus()}>
              Retry
            </ButtonPrimary>
          ) : null}
        </AppPanel>
      </ServerContext.Provider>
    );
  }

  return (
    <ServerContext.Provider
      value={{
        status: serverStatus,
        statusError: serverStatusError,
        info: serverInfo,
        workspaceDir,
        refreshStatus: fetchServerStatus,
      }}
    >
      {children}
    </ServerContext.Provider>
  );
}
