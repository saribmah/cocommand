import { create } from "zustand";
import { hideWindow } from "../../lib/ipc";
import type { StreamEvent } from "../session/session.types";
import type { MessagePart, MessagePartInput } from "./command.types";

type SendMessageFn = (
  parts: MessagePartInput[],
  onEvent?: (event: StreamEvent) => void
) => Promise<{ reply_parts?: MessagePart[] }>;

export interface CommandState {
  input: string;
  isSubmitting: boolean;
  parts: MessagePart[];
  error: string | null;
  setInput: (value: string) => void;
  clearInput: () => void;
  setError: (error: string | null) => void;
  reset: () => void;
  dismiss: () => void;
  submit: (sendMessage: SendMessageFn, override?: string) => Promise<boolean>;
}

export type CommandStore = ReturnType<typeof createCommandStore>;

export const createCommandStore = () => {
  return create<CommandState>()((set, get) => ({
    input: "",
    isSubmitting: false,
    parts: [],
    error: null,

    setInput: (value) => set({ input: value }),

    clearInput: () => set({ input: "" }),

    setError: (error) => set({ error }),

    reset: () =>
      set({
        input: "",
        isSubmitting: false,
        parts: [],
        error: null,
      }),

    dismiss: () => {
      const { parts, error } = get();
      if (parts.length > 0 || error) {
        set({ parts: [], error: null });
        return;
      }
      hideWindow();
    },

    submit: async (sendMessage, override) => {
      const text = (override ?? get().input).trim();
      if (!text) return false;

      set({ isSubmitting: true, parts: [], error: null });

      try {
        const streamParts: MessagePart[] = [];
        const inputParts = buildInputParts(text);

        const response = await sendMessage(inputParts, (event: StreamEvent) => {
          if (event.event !== "part.updated") return;
          const part = getPartFromEventData(event.data);
          if (!part) return;
          const existingIndex = findPartIndexToUpdate(streamParts, part);
          if (existingIndex >= 0) {
            streamParts[existingIndex] = part;
          } else {
            streamParts.push(part);
          }
          set({ parts: [...streamParts] });
        });

        set({
          input: "",
          isSubmitting: false,
          parts: response.reply_parts ?? [],
          error: null,
        });
        return true;
      } catch (err) {
        console.error("CommandStore submit error", err);
        set({
          isSubmitting: false,
          parts: [],
          error: normalizeErrorMessage(err),
        });
        return false;
      }
    },
  }));
};

function normalizeErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

function getPartFromEventData(value: unknown): MessagePart | null {
  if (!value || typeof value !== "object") return null;
  const part = (value as { part?: unknown }).part;
  if (!part || typeof part !== "object") return null;
  if (!("type" in part)) return null;
  return part as MessagePart;
}

function findPartIndexToUpdate(parts: MessagePart[], nextPart: MessagePart): number {
  const byId = parts.findIndex((part) => part.id === nextPart.id);
  if (byId >= 0) return byId;
  if (nextPart.type !== "tool") return -1;
  return parts.findIndex(
    (part) => part.type === "tool" && part.callId === nextPart.callId
  );
}

function buildInputParts(text: string): MessagePartInput[] {
  const extensionParts = extractExtensionParts(text);
  return [
    ...extensionParts,
    {
      type: "text",
      text,
    },
  ];
}

function extractExtensionParts(text: string): MessagePartInput[] {
  const parts: MessagePartInput[] = [];
  const seen = new Set<string>();
  const pattern = /(^|\s)@([^\s@]+)/g;

  let match: RegExpExecArray | null = pattern.exec(text);
  while (match) {
    const prefix = match[1] ?? "";
    const extensionId = (match[2] ?? "").trim();
    if (extensionId && !seen.has(extensionId)) {
      seen.add(extensionId);
      const sourceStart = match.index + prefix.length;
      const sourceValue = `@${extensionId}`;
      const sourceEnd = sourceStart + sourceValue.length;
      parts.push({
        type: "extension",
        extensionId,
        name: extensionId,
        source: {
          value: sourceValue,
          start: sourceStart,
          end: sourceEnd,
        },
      });
    }
    match = pattern.exec(text);
  }

  return parts;
}
