import { create } from "zustand";
import type { ApplicationInfo } from "../types/application";
import { useServerStore } from "./server";

interface ApplicationState {
  applications: ApplicationInfo[];
  isLoaded: boolean;
  fetchApplications: () => Promise<void>;
  openApplication: (id: string) => Promise<void>;
  getApplications: () => ApplicationInfo[];
}

function buildServerUrl(addr: string, path: string): string {
  const prefix = addr.startsWith("http") ? addr : `http://${addr}`;
  return `${prefix}${path}`;
}

export const useApplicationStore = create<ApplicationState>((set, get) => ({
  applications: [],
  isLoaded: false,
  fetchApplications: async () => {
    const server = useServerStore.getState().info;
    if (!server) {
      set({ applications: [], isLoaded: false });
      return;
    }
    const url = buildServerUrl(server.addr, "/workspace/applications");
    const response = await fetch(url);
    if (!response.ok) {
      set({ applications: [], isLoaded: false });
      return;
    }
    const data = (await response.json()) as ApplicationInfo[];
    set({ applications: data, isLoaded: true });
  },
  openApplication: async (id: string) => {
    const server = useServerStore.getState().info;
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
  getApplications: () => get().applications,
}));
