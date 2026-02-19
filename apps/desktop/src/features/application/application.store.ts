import { create } from "zustand";
import type { Sdk } from "@cocommand/sdk";
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

export type ApplicationStore = ReturnType<typeof createApplicationStore>;

export const createApplicationStore = (sdk: Sdk) => {
  return create<ApplicationState>()((set) => ({
    applications: [],
    count: 0,
    isLoaded: false,
    isLoading: false,
    isOpening: false,
    error: null,

    fetchApplications: async () => {
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
