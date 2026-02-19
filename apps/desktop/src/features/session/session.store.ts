import { create } from "zustand";
import {
  type ApiSessionContext,
  type RecordMessageResponse,
  type Sdk,
  type SessionCommandInputPart,
} from "@cocommand/sdk";

export interface StreamEvent<T = unknown> {
  event: string;
  data: T;
}

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
        if (event.type === "part.updated") {
          onEvent?.({
            event: "part.updated",
            data: {
              part_id: event.partId,
              part: event.part,
            },
          });
          continue;
        }

        if (event.type === "context") {
          set({ context: event.context });
          onEvent?.({
            event: "context",
            data: {
              context: event.context,
            },
          });
          continue;
        }

        if (event.type === "done") {
          finalResponse = {
            context: event.context,
            reply_parts: event.replyParts,
          };
          set({ context: event.context });
          onEvent?.({
            event: "done",
            data: {
              context: event.context,
              reply_parts: event.replyParts,
            },
          });
        }
      }

      if (!finalResponse) {
        throw new Error("Stream ended without a final response");
      }
      return finalResponse;
    },
    getContext: () => get().context,
  }));
};
