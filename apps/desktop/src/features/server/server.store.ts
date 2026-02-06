import { create } from "zustand";
import { getServerInfo, type ServerInfo } from "../../lib/ipc";

export interface ServerState {
  info: ServerInfo | null;
  status: "starting" | "ready" | "error";
  statusError: string | null;
  workspaceDir: string | null;
  setInfo: (info: ServerInfo) => void;
  clear: () => void;
  fetchStatus: () => Promise<void>;
  getInfo: () => ServerInfo | null;
}

export type ServerStore = ReturnType<typeof createServerStore>;

export const createServerStore = (serverInfo: ServerInfo) => {
  return create<ServerState>()((set, get) => ({
    info: serverInfo,
    status: serverInfo.status,
    statusError: serverInfo.error ?? null,
    workspaceDir: serverInfo.workspace_dir,
    setInfo: (info) =>
      set({
        info,
        status: info.status,
        statusError: info.error ?? null,
        workspaceDir: info.workspace_dir,
      }),
    clear: () => set({ info: null }),
    fetchStatus: async () => {
      try {
        const nextServerInfo = await getServerInfo();
        set({
          info: nextServerInfo,
          status: nextServerInfo.status,
          statusError: nextServerInfo.error ?? null,
          workspaceDir: nextServerInfo.workspace_dir,
        });
      } catch (error) {
        set({
          info: null,
          workspaceDir: null,
          status: "error",
          statusError: String(error),
        });
      }
    },
    getInfo: () => get().info,
  }));
};
