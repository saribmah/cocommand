import { create } from "zustand";
import type {
  UpdateWorkspaceSettingsPayload,
  WorkspaceSettings,
} from "../types/workspace";
import { useServerStore } from "./server";

interface WorkspaceState {
  settings: WorkspaceSettings | null;
  isLoaded: boolean;
  error: string | null;
  fetchSettings: () => Promise<void>;
  updateSettings: (
    payload: UpdateWorkspaceSettingsPayload,
  ) => Promise<WorkspaceSettings>;
}

function buildServerUrl(addr: string, path: string): string {
  const prefix = addr.startsWith("http") ? addr : `http://${addr}`;
  return `${prefix}${path}`;
}

export const useWorkspaceStore = create<WorkspaceState>((set) => ({
  settings: null,
  isLoaded: false,
  error: null,
  fetchSettings: async () => {
    const server = useServerStore.getState().info;
    if (!server) {
      set({ settings: null, isLoaded: false, error: null });
      return;
    }
    const url = buildServerUrl(server.addr, "/workspace/settings/workspace");
    try {
      const response = await fetch(url);
      if (!response.ok) {
        throw new Error(`Server error (${response.status})`);
      }
      const data = (await response.json()) as WorkspaceSettings;
      set({ settings: data, isLoaded: true, error: null });
    } catch (err) {
      set({ settings: null, isLoaded: false, error: String(err) });
    }
  },
  updateSettings: async (payload) => {
    const server = useServerStore.getState().info;
    if (!server) {
      throw new Error("Server unavailable");
    }
    const url = buildServerUrl(server.addr, "/workspace/settings/workspace");
    const response = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(errorText || `Server error (${response.status})`);
    }
    const data = (await response.json()) as WorkspaceSettings;
    set({ settings: data, isLoaded: true, error: null });
    return data;
  },
}));
