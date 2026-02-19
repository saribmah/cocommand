import { create } from "zustand";
import { hideWindow } from "../../lib/ipc";
import type { StreamEvent } from "../session/session.store";
import type {
  CommandTurn,
  MessagePart,
  MessagePartInput,
  RecordMessageResponse,
} from "./command.types";

type SendMessageFn = (
  parts: MessagePartInput[],
  onEvent?: (event: StreamEvent) => void
) => Promise<RecordMessageResponse>;

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
  const withoutBlankText = normalized.filter((part) =>
    part.type === "text" ? part.text.trim().length > 0 : true
  );
  return withoutBlankText;
}

function createTurnId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
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

export interface CommandState {
  draftParts: MessagePartInput[];
  isSubmitting: boolean;
  parts: MessagePart[];
  turns: CommandTurn[];
  error: string | null;
  setDraftParts: (parts: MessagePartInput[]) => void;
  setError: (error: string | null) => void;
  reset: () => void;
  dismiss: () => void;
  submit: (sendMessage: SendMessageFn) => Promise<boolean>;
}

export type CommandStore = ReturnType<typeof createCommandStore>;

export const createCommandStore = () => {
  return create<CommandState>()((set, get) => ({
    draftParts: [emptyTextPart()],
    isSubmitting: false,
    parts: [],
    turns: [],
    error: null,

    setDraftParts: (parts) => set({ draftParts: normalizeDraftParts(parts) }),

    setError: (error) => set({ error }),

    reset: () =>
      set({
        draftParts: [emptyTextPart()],
        isSubmitting: false,
        parts: [],
        error: null,
      }),

    dismiss: () => {
      set({ parts: [], error: null });
      hideWindow();
    },

    submit: async (sendMessage) => {
      const draftParts = get().draftParts;
      const inputParts = submitReadyParts(draftParts);
      if (inputParts.length === 0 || inputParts.every(isBlankTextPart)) {
        return false;
      }

      const turnId = createTurnId();
      const turnInputParts = cloneMessagePartInputs(inputParts);
      set((state) => ({
        isSubmitting: true,
        parts: [],
        error: null,
        turns: [
          ...state.turns,
          {
            id: turnId,
            submittedAt: Date.now(),
            inputParts: turnInputParts,
            replyParts: [],
            status: "streaming",
            error: null,
          },
        ],
      }));

      try {
        const streamParts: MessagePart[] = [];
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
          set((state) => ({
            parts: [...streamParts],
            turns: updateTurnById(state.turns, turnId, (turn) => ({
              ...turn,
              replyParts: [...streamParts],
            })),
          }));
        });

        const replyParts = response.reply_parts ?? [];
        set((state) => ({
          draftParts: [emptyTextPart()],
          isSubmitting: false,
          parts: replyParts,
          error: null,
          turns: updateTurnById(state.turns, turnId, (turn) => ({
            ...turn,
            replyParts,
            status: "complete",
            error: null,
          })),
        }));
        return true;
      } catch (err) {
        console.error("CommandStore submit error", err);
        const errorMessage = normalizeErrorMessage(err);
        set((state) => ({
          isSubmitting: false,
          parts: [],
          error: null,
          turns: updateTurnById(state.turns, turnId, (turn) => ({
            ...turn,
            status: "error",
            error: errorMessage,
          })),
        }));
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

function updateTurnById(
  turns: CommandTurn[],
  turnId: string,
  updater: (turn: CommandTurn) => CommandTurn
): CommandTurn[] {
  return turns.map((turn) => (turn.id === turnId ? updater(turn) : turn));
}
