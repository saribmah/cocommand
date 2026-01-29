import { create } from "zustand";
import type { SessionContext } from "../types/session";
import { useServerStore } from "./server";

interface SessionState {
  context: SessionContext | null;
  setContext: (context: SessionContext) => void;
  clear: () => void;
  sendMessage: (text: string) => Promise<SessionContext>;
}

function buildServerUrl(addr: string, path: string): string {
  const prefix = addr.startsWith("http") ? addr : `http://${addr}`;
  return `${prefix}${path}`;
}

export const useSessionStore = create<SessionState>((set) => ({
  context: null,
  setContext: (context) => set({ context }),
  clear: () => set({ context: null }),
  sendMessage: async (text) => {
    const server = useServerStore.getState().info;
    if (!server) {
      throw new Error("Server unavailable");
    }
    const url = buildServerUrl(server.addr, "/sessions/message");
    const response = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ text }),
    });
    if (!response.ok) {
      throw new Error(`Server error (${response.status})`);
    }
    const data = (await response.json()) as SessionContext;
    set({ context: data });
    return data;
  },
}));
