import { create } from "zustand";
import { hideWindow } from "../../lib/ipc";
import type {
  EnqueueMessageResponse,
  RuntimeEvent,
} from "@cocommand/sdk";
import type {
  Message,
  MessagePart,
  MessagePartInput,
} from "./command.types";

type SendMessageFn = (
  parts: MessagePartInput[],
) => Promise<EnqueueMessageResponse>;

function emptyTextPart(): MessagePartInput {
  return { type: "text", text: "" };
}

function isBlankTextPart(part: MessagePartInput): boolean {
  return part.type === "text" && part.text.trim().length === 0;
}

function normalizeDraftParts(parts: MessagePartInput[]): MessagePartInput[] {
  const normalized: MessagePartInput[] = [];
  for (const part of parts) {
    const previous = normalized[normalized.length - 1];
    if (part.type === "text" && previous?.type === "text") {
      previous.text += part.text;
      continue;
    }
    normalized.push(part);
  }
  if (normalized.length === 0) {
    return [emptyTextPart()];
  }
  return normalized;
}

function submitReadyParts(parts: MessagePartInput[]): MessagePartInput[] {
  const normalized = normalizeDraftParts(parts);
  return normalized.filter((part) =>
    part.type === "text" ? part.text.trim().length > 0 : true
  );
}

function cloneMessagePartInput(part: MessagePartInput): MessagePartInput {
  switch (part.type) {
    case "text":
      return { ...part };
    case "extension":
      return {
        ...part,
        source: part.source ? { ...part.source } : part.source,
      };
    case "file":
      return {
        ...part,
        source: part.source ? { ...part.source } : part.source,
      };
    default:
      return part;
  }
}

function cloneMessagePartInputs(parts: MessagePartInput[]): MessagePartInput[] {
  return parts.map(cloneMessagePartInput);
}

function findPartIndexToUpdate(parts: MessagePart[], nextPart: MessagePart): number {
  const byId = parts.findIndex((part) => part.id === nextPart.id);
  if (byId >= 0) return byId;
  if (nextPart.type !== "tool") return -1;
  return parts.findIndex(
    (part) => part.type === "tool" && part.callId === nextPart.callId
  );
}

function sortMessagesChronologically(messages: Message[]): Message[] {
  return [...messages].sort((left, right) =>
    left.info.createdAt.localeCompare(right.info.createdAt)
  );
}

function mergeMessages(existing: Message[], incoming: Message[]): Message[] {
  if (incoming.length === 0) return existing;
  const merged = [...existing];
  for (const nextMessage of incoming) {
    const index = merged.findIndex(
      (message) => message.info.id === nextMessage.info.id
    );
    if (index >= 0) {
      merged[index] = nextMessage;
    } else {
      merged.push(nextMessage);
    }
  }
  return sortMessagesChronologically(merged);
}

function mergeHistoryMessages(existing: Message[], incoming: Message[]): Message[] {
  if (incoming.length === 0) return existing;
  if (existing.length === 0) return sortMessagesChronologically(incoming);

  const existingIds = new Set(existing.map((message) => message.info.id));
  const merged = [...existing];
  for (const nextMessage of incoming) {
    if (!existingIds.has(nextMessage.info.id)) {
      merged.push(nextMessage);
    }
  }
  return sortMessagesChronologically(merged);
}

function updateMessagePart(
  messages: Message[],
  messageId: string,
  nextPart: MessagePart
): Message[] {
  return messages.map((message) => {
    if (message.info.id !== messageId) {
      return message;
    }
    const existingIndex = findPartIndexToUpdate(message.parts, nextPart);
    if (existingIndex >= 0) {
      const parts = [...message.parts];
      parts[existingIndex] = nextPart;
      return { ...message, parts };
    }
    return { ...message, parts: [...message.parts, nextPart] };
  });
}

export interface CommandState {
  draftParts: MessagePartInput[];
  submittedInputHistory: MessagePartInput[][];
  isSubmitting: boolean;
  messages: Message[];
  error: string | null;
  setDraftParts: (parts: MessagePartInput[]) => void;
  setError: (error: string | null) => void;
  hydrateMessages: (messages: Message[]) => void;
  applyRuntimeEvent: (event: RuntimeEvent) => void;
  reset: () => void;
  dismiss: () => void;
  submit: (sendMessage: SendMessageFn) => Promise<boolean>;
}

export type CommandStore = ReturnType<typeof createCommandStore>;

export const createCommandStore = () => {
  return create<CommandState>()((set, get) => ({
    draftParts: [emptyTextPart()],
    submittedInputHistory: [],
    isSubmitting: false,
    messages: [],
    error: null,

    setDraftParts: (parts) => set({ draftParts: normalizeDraftParts(parts) }),

    setError: (error) => set({ error }),

    hydrateMessages: (messages) =>
      set((state) => ({
        messages: mergeHistoryMessages(state.messages, messages),
      })),

    applyRuntimeEvent: (event) => {
      if (event.type === "message.started") {
        const incoming = event.userMessage
          ? [event.userMessage, event.assistantMessage]
          : [event.assistantMessage];
        set((state) => ({
          messages: mergeMessages(state.messages, incoming),
        }));
        return;
      }

      if (event.type === "part.updated") {
        set((state) => ({
          messages: updateMessagePart(state.messages, event.messageId, event.part),
        }));
      }
    },

    reset: () =>
      set({
        draftParts: [emptyTextPart()],
        isSubmitting: false,
        error: null,
      }),

    dismiss: () => {
      set({ error: null });
      hideWindow();
    },

    submit: async (sendMessage) => {
      const draftParts = get().draftParts;
      const inputParts = submitReadyParts(draftParts);
      if (inputParts.length === 0 || inputParts.every(isBlankTextPart)) {
        return false;
      }

      const submittedInput = cloneMessagePartInputs(inputParts);
      set((state) => ({
        isSubmitting: true,
        error: null,
        submittedInputHistory: [...state.submittedInputHistory, submittedInput],
      }));

      try {
        await sendMessage(inputParts);

        set({
          draftParts: [emptyTextPart()],
          isSubmitting: false,
          error: null,
        });
        return true;
      } catch (err) {
        console.error("CommandStore submit error", err);
        const errorMessage = normalizeErrorMessage(err);
        set({
          isSubmitting: false,
          error: errorMessage,
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
