import { create } from "zustand";
import type { ExtensionInvokeFn } from "../extension/extension.types";
import type {
  WorkspaceConfig,
  WorkspacePermissionStatus,
  WorkspacePermissionsResponse,
} from "./workspace.types";

export interface WorkspaceExtensionState {
  config: WorkspaceConfig | null;
  permissions: WorkspacePermissionStatus[];
  permissionsPlatform: string | null;
  isLoading: boolean;
  isSaving: boolean;
  error: string | null;
  fetchConfig: () => Promise<void>;
  updateConfig: (config: WorkspaceConfig) => Promise<WorkspaceConfig>;
  fetchPermissions: () => Promise<void>;
  openPermission: (id: string) => Promise<void>;
}

export type WorkspaceExtensionStore = ReturnType<typeof createWorkspaceExtensionStore>;

export const createWorkspaceExtensionStore = (invoke: ExtensionInvokeFn) => {
  return create<WorkspaceExtensionState>()((set) => ({
    config: null,
    permissions: [],
    permissionsPlatform: null,
    isLoading: false,
    isSaving: false,
    error: null,

    fetchConfig: async () => {
      set({ isLoading: true, error: null });
      try {
        const data = await invoke<WorkspaceConfig>("workspace", "get_config");
        set({ config: data, isLoading: false, error: null });
      } catch (error) {
        set({ config: null, isLoading: false, error: String(error) });
      }
    },

    updateConfig: async (config: WorkspaceConfig) => {
      set({ isSaving: true, error: null });
      try {
        const data = await invoke<WorkspaceConfig>(
          "workspace",
          "update_config",
          { config },
        );
        set({ config: data, isSaving: false, error: null });
        return data;
      } catch (error) {
        set({ isSaving: false, error: String(error) });
        throw error;
      }
    },

    fetchPermissions: async () => {
      try {
        const data = await invoke<WorkspacePermissionsResponse>(
          "workspace",
          "get_permissions",
        );
        set({
          permissionsPlatform: data.platform,
          permissions: data.permissions,
        });
      } catch (error) {
        set({ permissions: [], error: String(error) });
      }
    },

    openPermission: async (id: string) => {
      await invoke("workspace", "open_permission", { id });
    },
  }));
};
