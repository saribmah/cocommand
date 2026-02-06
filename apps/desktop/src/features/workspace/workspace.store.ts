import { create } from "zustand";
import type { ServerInfo } from "../../lib/ipc";
import type { WorkspaceConfig } from "./workspace.types";

export interface WorkspaceState {
  config: WorkspaceConfig | null;
  isLoaded: boolean;
  error: string | null;
  fetchConfig: () => Promise<void>;
  updateConfig: (config: WorkspaceConfig) => Promise<WorkspaceConfig>;
}

function buildServerUrl(addr: string, path: string): string {
  const prefix = addr.startsWith("http") ? addr : `http://${addr}`;
  return `${prefix}${path}`;
}

export type WorkspaceStore = ReturnType<typeof createWorkspaceStore>;

export const createWorkspaceStore = (getServer: () => ServerInfo | null) => {
  return create<WorkspaceState>()((set) => ({
    config: null,
    isLoaded: false,
    error: null,
    fetchConfig: async () => {
      const server = getServer();
      if (!server) {
        set({ config: null, isLoaded: false, error: null });
        return;
      }
      const url = buildServerUrl(server.addr, "/workspace/config");
      try {
        const response = await fetch(url);
        if (!response.ok) {
          throw new Error(`Server error (${response.status})`);
        }
        const data = (await response.json()) as WorkspaceConfig;
        set({ config: data, isLoaded: true, error: null });
      } catch (error) {
        set({ config: null, isLoaded: false, error: String(error) });
      }
    },
    updateConfig: async (config) => {
      const server = getServer();
      if (!server) {
        throw new Error("Server unavailable");
      }
      const url = buildServerUrl(server.addr, "/workspace/config");
      const response = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(config),
      });
      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(errorText || `Server error (${response.status})`);
      }
      const data = (await response.json()) as WorkspaceConfig;
      set({ config: data, isLoaded: true, error: null });
      return data;
    },
  }));
};
