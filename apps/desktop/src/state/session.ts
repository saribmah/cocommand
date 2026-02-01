import { create } from "zustand";
import type {
  RecordMessageResponse,
  SessionContext,
  StreamEvent,
} from "../types/session";
import { useServerStore } from "./server";

interface SessionState {
  context: SessionContext | null;
  setContext: (context: SessionContext) => void;
  clear: () => void;
  sendMessage: (
    text: string,
    onEvent?: (event: StreamEvent) => void
  ) => Promise<RecordMessageResponse>;
  getContext: () => SessionContext | null;
}

function buildServerUrl(addr: string, path: string): string {
  const prefix = addr.startsWith("http") ? addr : `http://${addr}`;
  return `${prefix}${path}`;
}

export const useSessionStore = create<SessionState>((set, get) => ({
  context: null,
  setContext: (context) => set({ context }),
  clear: () => set({ context: null }),
  sendMessage: async (text, onEvent) => {
    const server = useServerStore.getState().info;
    if (!server) {
      throw new Error("Server unavailable");
    }
    const url = buildServerUrl(server.addr, "/sessions/message/stream");
    const response = await fetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Accept: "text/event-stream",
      },
      body: JSON.stringify({ text }),
    });
    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(errorText || `Server error (${response.status})`);
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

        if (parsed.event === "context" && parsed.data?.context) {
          set({ context: parsed.data.context });
        }
        if (parsed.event === "done" && parsed.data?.reply_parts) {
          finalResponse = {
            context: parsed.data.context,
            reply_parts: parsed.data.reply_parts,
          };
          if (finalResponse.context) {
            set({ context: finalResponse.context });
          }
        }
        if (parsed.event === "error") {
          throw new Error(parsed.data?.error || "Streaming error");
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
