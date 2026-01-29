import { create } from "zustand";
import { getServerInfo, type ServerInfo } from "../lib/ipc";

interface ServerState {
  info: ServerInfo | null;
  setInfo: (info: ServerInfo) => void;
  clear: () => void;
  fetchInfo: () => Promise<void>;
  getInfo: () => ServerInfo | null;
}

export const useServerStore = create<ServerState>((set, get) => ({
  info: null,
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
  getInfo: () => get().info,
}));
