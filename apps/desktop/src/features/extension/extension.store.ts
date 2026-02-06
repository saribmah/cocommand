import { create } from "zustand";
import type { ServerInfo } from "../../lib/ipc";
import type { ApplicationInfo } from "../../types/application";

export type ExtensionInfo = ApplicationInfo;

export interface ExtensionState {
  extensions: ExtensionInfo[];
  isLoaded: boolean;
  error: string | null;
  fetchExtensions: () => Promise<void>;
  openExtension: (id: string) => Promise<void>;
  getExtensions: () => ExtensionInfo[];
}

function buildServerUrl(addr: string, path: string): string {
  const prefix = addr.startsWith("http") ? addr : `http://${addr}`;
  return `${prefix}${path}`;
}

export type ExtensionStore = ReturnType<typeof createExtensionStore>;

export const createExtensionStore = (getServer: () => ServerInfo | null) => {
  return create<ExtensionState>()((set, get) => ({
    extensions: [],
    isLoaded: false,
    error: null,
    fetchExtensions: async () => {
      const server = getServer();
      if (!server) {
        set({ extensions: [], isLoaded: false, error: null });
        return;
      }

      const url = buildServerUrl(server.addr, "/workspace/applications");
      try {
        const response = await fetch(url);
        if (!response.ok) {
          throw new Error(`Server error (${response.status})`);
        }
        const data = (await response.json()) as ExtensionInfo[];
        set({ extensions: data, isLoaded: true, error: null });
      } catch (error) {
        set({ extensions: [], isLoaded: false, error: String(error) });
      }
    },
    openExtension: async (id) => {
      const server = getServer();
      if (!server) {
        throw new Error("Server unavailable");
      }

      const url = buildServerUrl(server.addr, "/workspace/applications/open");
      const response = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id }),
      });
      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(errorText || `Server error (${response.status})`);
      }
    },
    getExtensions: () => get().extensions,
  }));
};
