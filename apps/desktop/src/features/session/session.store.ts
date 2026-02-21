import { create } from "zustand";
import {
  type ApiSessionContext,
  type EnqueueMessageResponse,
  type Message,
  type Sdk,
  type SessionCommandInputPart,
} from "@cocommand/sdk";

export interface SessionState {
  context: ApiSessionContext | null;
  setContext: (context: ApiSessionContext) => void;
  clear: () => void;
  sendMessage: (
    parts: SessionCommandInputPart[],
  ) => Promise<EnqueueMessageResponse>;
  loadMessageHistory: () => Promise<Message[]>;
  getContext: () => ApiSessionContext | null;
}

export type SessionStore = ReturnType<typeof createSessionStore>;

export const createSessionStore = (sdk: Sdk) => {
  return create<SessionState>()((set, get) => ({
    context: null,
    setContext: (context) => set({ context }),
    clear: () => set({ context: null }),
    sendMessage: async (parts) => {
      const response = await sdk.sessions.command(parts);
      set({ context: response.context });
      return response;
    },
    loadMessageHistory: async () => sdk.sessions.history(),
    getContext: () => get().context,
  }));
};
