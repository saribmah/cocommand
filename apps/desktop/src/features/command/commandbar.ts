import { useCallback, useState } from "react";
import { hideWindow, openSettingsWindow } from "../../lib/ipc";
import type { StreamEvent } from "../session/session.types";
import type { MessagePart } from "./command.types";

export interface CommandBarState {
  input: string;
  isSubmitting: boolean;
  parts: MessagePart[];
  error: string | null;
}

type SendMessageFn = (
  text: string,
  onEvent?: (event: StreamEvent) => void
) => Promise<{ reply_parts?: MessagePart[] }>;

export function useCommandBar(sendMessage: SendMessageFn) {
  const [state, setState] = useState<CommandBarState>({
    input: "",
    isSubmitting: false,
    parts: [],
    error: null,
  });

  const setInput = useCallback((value: string) => {
    setState((s) => ({ ...s, input: value }));
  }, []);

  const setError = useCallback((error: string | null) => {
    setState((s) => ({ ...s, error }));
  }, []);

  const reset = useCallback(() => {
    setState({
      input: "",
      isSubmitting: false,
      parts: [],
      error: null,
    });
  }, []);

  const submit = useCallback(async (override?: string): Promise<boolean> => {
    const text = (override ?? state.input).trim();
    if (!text) return false;

    if (text === "/settings") {
      await openSettingsWindow();
      hideWindow();
      setState((s) => ({ ...s, input: "", parts: [], isSubmitting: false, error: null }));
      return true;
    }

    setState((s) => ({ ...s, isSubmitting: true, parts: [], error: null }));

    try {
      const streamParts: MessagePart[] = [];

      const response = await sendMessage(text, (event: StreamEvent) => {
        if (event.event !== "part.updated") return;
        const part = getPartFromEventData(event.data);
        if (!part) return;
        const existingIndex = findPartIndexToUpdate(streamParts, part);
        if (existingIndex >= 0) {
          streamParts[existingIndex] = part;
        } else {
          streamParts.push(part);
        }
        setState((s) => ({ ...s, parts: [...streamParts] }));
      });

      setState((s) => ({
        ...s,
        input: "",
        isSubmitting: false,
        parts: response.reply_parts ?? [],
        error: null,
      }));
      return true;
    } catch (err) {
      console.error("CommandBar submit error", err);
      setState((s) => ({
        ...s,
        isSubmitting: false,
        parts: [],
        error: normalizeErrorMessage(err),
      }));
      return false;
    }
  }, [state.input, sendMessage]);

  const dismiss = useCallback(() => {
    if (state.parts.length > 0 || state.error) {
      setState((s) => ({ ...s, parts: [], error: null }));
    } else {
      hideWindow();
    }
  }, [state.parts.length, state.error]);

  return {
    ...state,
    setInput,
    setError,
    submit,
    dismiss,
    reset,
  };
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
