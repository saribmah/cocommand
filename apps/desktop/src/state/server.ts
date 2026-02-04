import { create } from "zustand";
import { getServerInfo, getServerStatus, type ServerInfo } from "../lib/ipc";

interface ServerState {
  info: ServerInfo | null;
  status: "starting" | "ready" | "error";
  statusError: string | null;
  workspaceDir: string | null;
  setInfo: (info: ServerInfo) => void;
  clear: () => void;
  fetchInfo: () => Promise<void>;
  fetchStatus: () => Promise<void>;
  getInfo: () => ServerInfo | null;
}

export const useServerStore = create<ServerState>((set, get) => ({
  info: null,
  status: "starting",
  statusError: null,
  workspaceDir: null,
  setInfo: (info) => set({ info }),
  clear: () => set({ info: null }),
  fetchInfo: async () => {
    try {
      const info = await getServerInfo();
      set({ info });
    } catch {
      set({ info: null });
    }
  },
  fetchStatus: async () => {
    try {
      const status = await getServerStatus();
      set({
        status: status.status,
        statusError: status.error ?? null,
        workspaceDir: status.workspace_dir,
      });
    } catch (error) {
      set({
        status: "error",
        statusError: String(error),
      });
    }
  },
  getInfo: () => get().info,
}));
