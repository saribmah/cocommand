import { create } from "zustand";
import type { ServerInfo } from "../../lib/ipc";
import type {
  ApplicationInfo,
  ApplicationsResponse,
  OpenApplicationRequest,
  OpenApplicationResponse,
} from "./application.types";

export interface ApplicationState {
  applications: ApplicationInfo[];
  count: number;
  isLoaded: boolean;
  isLoading: boolean;
  isOpening: boolean;
  error: string | null;
  fetchApplications: () => Promise<void>;
  openApplication: (request: OpenApplicationRequest) => Promise<OpenApplicationResponse>;
  clear: () => void;
}

function buildServerUrl(addr: string, path: string): string {
  const prefix = addr.startsWith("http") ? addr : `http://${addr}`;
  return `${prefix}${path}`;
}

export type ApplicationStore = ReturnType<typeof createApplicationStore>;

export const createApplicationStore = (getServer: () => ServerInfo | null) => {
  return create<ApplicationState>()((set) => ({
    applications: [],
    count: 0,
    isLoaded: false,
    isLoading: false,
    isOpening: false,
    error: null,

    fetchApplications: async () => {
      const server = getServer();
      if (!server || !server.addr) {
        set({
          applications: [],
          count: 0,
          isLoaded: false,
          isLoading: false,
          error: "Server unavailable",
        });
        return;
      }

      set({ isLoading: true, error: null });
      const url = buildServerUrl(server.addr, "/workspace/extension/system/applications");
      try {
        const response = await fetch(url);
        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(errorText || `Server error (${response.status})`);
        }
        const data = (await response.json()) as ApplicationsResponse;
        set({
          applications: data.applications,
          count: data.count,
          isLoaded: true,
          isLoading: false,
          error: null,
        });
      } catch (error) {
        set({
          applications: [],
          count: 0,
          isLoaded: false,
          isLoading: false,
          error: String(error),
        });
      }
    },

    openApplication: async (request) => {
      const server = getServer();
      if (!server || !server.addr) {
        throw new Error("Server unavailable");
      }

      set({ isOpening: true, error: null });
      const url = buildServerUrl(
        server.addr,
        "/workspace/extension/system/applications/open"
      );
      try {
        const response = await fetch(url, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(request),
        });
        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(errorText || `Server error (${response.status})`);
        }
        const data = (await response.json()) as OpenApplicationResponse;
        set({ isOpening: false, error: null });
        return data;
      } catch (error) {
        const message = String(error);
        set({ isOpening: false, error: message });
        throw error;
      }
    },

    clear: () =>
      set({
        applications: [],
        count: 0,
        isLoaded: false,
        isLoading: false,
        isOpening: false,
        error: null,
      }),
  }));
};
