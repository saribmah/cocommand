import { create } from "zustand";
import type { ApplicationInfo } from "../types/application";
import { useServerStore } from "./server";

interface ApplicationState {
  applications: ApplicationInfo[];
  isLoaded: boolean;
  fetchApplications: () => Promise<void>;
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
  getApplications: () => get().applications,
}));
