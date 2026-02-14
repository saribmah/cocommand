import { create } from "zustand";
import type { ServerInfo } from "../../lib/ipc";
import type {
  MessagePart,
  MessagePartInput,
  RecordMessageResponse,
} from "../command/command.types";
import type { SessionContext, StreamEvent } from "./session.types";

export interface SessionState {
  context: SessionContext | null;
  setContext: (context: SessionContext) => void;
  clear: () => void;
  sendMessage: (
    parts: MessagePartInput[],
    onEvent?: (event: StreamEvent) => void
  ) => Promise<RecordMessageResponse>;
  getContext: () => SessionContext | null;
}

type StreamPayload = {
  context?: SessionContext;
  reply_parts?: MessagePart[];
  error?: unknown;
};

function buildServerUrl(addr: string, path: string): string {
  const prefix = addr.startsWith("http") ? addr : `http://${addr}`;
  return `${prefix}${path}`;
}

function normalizeErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

function parseSseEvent(raw: string): StreamEvent | null {
  if (!raw.trim()) return null;
  let event = "message";
  const dataLines: string[] = [];
  const lines = raw.split("\n");
  for (const line of lines) {
    if (line.startsWith("event:")) {
      event = line.slice(6).trim();
    } else if (line.startsWith("data:")) {
      dataLines.push(line.slice(5).trimStart());
    }
  }
  const dataText = dataLines.join("\n");
  let data: unknown = dataText;
  if (dataText) {
    try {
      data = JSON.parse(dataText);
    } catch {
      data = dataText;
    }
  }
  return { event, data };
}

function asStreamPayload(value: unknown): StreamPayload | null {
  if (!value || typeof value !== "object") return null;
  return value as StreamPayload;
}

export type SessionStore = ReturnType<typeof createSessionStore>;

export const createSessionStore = (getServer: () => ServerInfo | null) => {
  return create<SessionState>()((set, get) => ({
    context: null,
    setContext: (context) => set({ context }),
    clear: () => set({ context: null }),
    sendMessage: async (parts, onEvent) => {
      const server = getServer();
      if (!server || !server.addr) {
        throw new Error("Server unavailable");
      }

      const url = buildServerUrl(server.addr, "/sessions/command");
      const response = await fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Accept: "text/event-stream",
        },
        body: JSON.stringify({ parts }),
      });

      if (!response.ok) {
        const errorText = await response.text();
        let message = errorText || `Server error (${response.status})`;
        if (errorText) {
          try {
            const parsed = JSON.parse(errorText) as { error?: unknown };
            if (parsed.error) {
              message = normalizeErrorMessage(parsed.error);
            }
          } catch {
            message = errorText;
          }
        }
        throw new Error(message);
      }

      const reader = response.body?.getReader();
      if (!reader) {
        throw new Error("Missing response stream");
      }

      const decoder = new TextDecoder();
      let buffer = "";
      let finalResponse: RecordMessageResponse | null = null;

      while (true) {
        const { value, done } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });
        buffer = buffer.replace(/\r\n/g, "\n");

        let splitIndex = buffer.indexOf("\n\n");
        while (splitIndex !== -1) {
          const rawEvent = buffer.slice(0, splitIndex);
          buffer = buffer.slice(splitIndex + 2);
          splitIndex = buffer.indexOf("\n\n");

          const parsed = parseSseEvent(rawEvent);
          if (!parsed) continue;
          if (onEvent) {
            onEvent(parsed);
          }

          const payload = asStreamPayload(parsed.data);
          if (parsed.event === "context" && payload?.context) {
            set({ context: payload.context });
          }

          if (parsed.event === "done" && payload?.reply_parts) {
            const context = payload.context ?? get().context;
            if (context) {
              finalResponse = {
                context,
                reply_parts: payload.reply_parts,
              };
              set({ context });
            }
          }

          if (parsed.event === "error") {
            const message = normalizeErrorMessage(payload?.error ?? parsed.data);
            console.error("SSE error event", parsed.data ?? parsed);
            throw new Error(message || "Streaming error");
          }
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
