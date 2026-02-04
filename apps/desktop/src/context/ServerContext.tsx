import { createContext, useContext } from "react";
import type { ServerInfo } from "../lib/ipc";

export interface ServerContextValue {
  status: "starting" | "ready" | "error";
  statusError: string | null;
  info: ServerInfo | null;
  workspaceDir: string | null;
  refreshStatus: () => Promise<void>;
}

export const ServerContext = createContext<ServerContextValue | null>(null);

export function useServerContext(): ServerContextValue {
  const context = useContext(ServerContext);
  if (!context) {
    throw new Error("useServerContext must be used within ServerContext provider");
  }
  return context;
}
