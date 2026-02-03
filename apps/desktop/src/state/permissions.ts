import { create } from "zustand";
import type { PermissionsResponse } from "../types/permissions";
import { useServerStore } from "./server";

interface PermissionsState {
  platform: string | null;
  permissions: PermissionsResponse["permissions"];
  isLoaded: boolean;
  error: string | null;
  fetchStatus: () => Promise<void>;
  openPermission: (id: string) => Promise<void>;
}

function buildServerUrl(addr: string, path: string): string {
  const prefix = addr.startsWith("http") ? addr : `http://${addr}`;
  return `${prefix}${path}`;
}

export const usePermissionsStore = create<PermissionsState>((set) => ({
  platform: null,
  permissions: [],
  isLoaded: false,
  error: null,
  fetchStatus: async () => {
    const server = useServerStore.getState().info;
    if (!server) {
      set({ platform: null, permissions: [], isLoaded: false, error: null });
      return;
    }
    const url = buildServerUrl(server.addr, "/workspace/settings/permissions");
    try {
      const response = await fetch(url);
      if (!response.ok) {
        throw new Error(`Server error (${response.status})`);
      }
      const data = (await response.json()) as PermissionsResponse;
      set({
        platform: data.platform,
        permissions: data.permissions,
        isLoaded: true,
        error: null,
      });
    } catch (err) {
      set({
        platform: null,
        permissions: [],
        isLoaded: false,
        error: String(err),
      });
    }
  },
  openPermission: async (id) => {
    const server = useServerStore.getState().info;
    if (!server) {
      throw new Error("Server unavailable");
    }
    const url = buildServerUrl(server.addr, "/workspace/settings/permissions/open");
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
}));
