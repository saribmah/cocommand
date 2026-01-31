import { create } from "zustand";
import type { AiSettings, UpdateAiSettingsPayload } from "../types/ai";
import { useServerStore } from "./server";

interface AiState {
  settings: AiSettings | null;
  isLoaded: boolean;
  error: string | null;
  fetchSettings: () => Promise<void>;
  updateSettings: (payload: UpdateAiSettingsPayload) => Promise<AiSettings>;
}

function buildServerUrl(addr: string, path: string): string {
  const prefix = addr.startsWith("http") ? addr : `http://${addr}`;
  return `${prefix}${path}`;
}

export const useAiStore = create<AiState>((set, get) => ({
  settings: null,
  isLoaded: false,
  error: null,
  fetchSettings: async () => {
    const server = useServerStore.getState().info;
    if (!server) {
      set({ settings: null, isLoaded: false, error: null });
      return;
    }
    const url = buildServerUrl(server.addr, "/workspace/settings/ai");
    try {
      const response = await fetch(url);
      if (!response.ok) {
        throw new Error(`Server error (${response.status})`);
      }
      const data = (await response.json()) as AiSettings;
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
    const url = buildServerUrl(server.addr, "/workspace/settings/ai");
    const response = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(errorText || `Server error (${response.status})`);
    }
    const data = (await response.json()) as AiSettings;
    set({ settings: data, isLoaded: true, error: null });
    return data;
  },
}));
