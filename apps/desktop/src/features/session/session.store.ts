import { create } from "zustand";
import {
  type ApiSessionContext,
  type RecordMessageResponse,
  type Sdk,
  type SessionCommandEvent,
  type SessionCommandInputPart,
} from "@cocommand/sdk";

export type StreamEvent = SessionCommandEvent;

export interface SessionState {
  context: ApiSessionContext | null;
  setContext: (context: ApiSessionContext) => void;
  clear: () => void;
  sendMessage: (
    parts: SessionCommandInputPart[],
    onEvent?: (event: StreamEvent) => void
  ) => Promise<RecordMessageResponse>;
  getContext: () => ApiSessionContext | null;
}

export type SessionStore = ReturnType<typeof createSessionStore>;

export const createSessionStore = (sdk: Sdk) => {
  return create<SessionState>()((set, get) => ({
    context: null,
    setContext: (context) => set({ context }),
    clear: () => set({ context: null }),
    sendMessage: async (parts, onEvent) => {
      let finalResponse: RecordMessageResponse | null = null;

      for await (const event of sdk.sessions.commandStream(parts)) {
        if (event.type === "context") {
          set({ context: event.context });
        }

        if (event.type === "done") {
          finalResponse = {
            context: event.context,
            messages: event.messages,
          };
          set({ context: event.context });
        }

        onEvent?.(event);
      }

      if (!finalResponse) {
        throw new Error("Stream ended without a final response");
      }
      return finalResponse;
    },
    getContext: () => get().context,
  }));
};
