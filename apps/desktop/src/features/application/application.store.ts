import { create } from "zustand";
import type { ServerInfo } from "../../lib/ipc";
import { createApiClient } from "@cocommand/api";
import { listApplications, openApplication } from "@cocommand/api";
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

function getClient(getServer: () => ServerInfo | null) {
  const server = getServer();
  if (!server?.addr) return null;
  return createApiClient(server.addr);
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
      const client = getClient(getServer);
      if (!client) {
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
      try {
        const { data, error: fetchError } = await listApplications({ client });
        if (fetchError) {
          throw new Error(fetchError.error?.message ?? "Server error");
        }
        const response = data as ApplicationsResponse;
        set({
          applications: response.applications,
          count: response.count,
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
      const client = getClient(getServer);
      if (!client) {
        throw new Error("Server unavailable");
      }

      set({ isOpening: true, error: null });
      try {
        const { data, error: fetchError } = await openApplication({
          client,
          body: request,
        });
        if (fetchError) {
          throw new Error(fetchError.error?.message ?? "Server error");
        }
        set({ isOpening: false, error: null });
        return data as OpenApplicationResponse;
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
