import { create } from "zustand";
import type { ServerInfo } from "../../lib/ipc";
import { createSdk, createSdkClient } from "@cocommand/sdk";
import type {
  ApplicationInfo,
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

function getSdk(getServer: () => ServerInfo | null) {
  const server = getServer();
  if (!server?.addr) return null;
  return createSdk({ client: createSdkClient(server.addr) });
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
      const sdk = getSdk(getServer);
      if (!sdk) {
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
        const applications = await sdk.applications.list();
        set({
          applications,
          count: applications.length,
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
      const sdk = getSdk(getServer);
      if (!sdk) {
        throw new Error("Server unavailable");
      }

      set({ isOpening: true, error: null });
      try {
        const response = await sdk.applications.open(request.id);
        set({ isOpening: false, error: null });
        return response as OpenApplicationResponse;
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
