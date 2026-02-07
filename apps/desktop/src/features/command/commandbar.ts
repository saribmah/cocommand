import { useCallback, useState } from "react";
import { hideWindow, openSettingsWindow } from "../../lib/ipc";
import type { MessagePart, StreamEvent } from "../session/session.types";

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

  const submit = useCallback(async (override?: string) => {
    const text = (override ?? state.input).trim();
    if (!text) return;

    if (text === "/settings") {
      await openSettingsWindow();
      hideWindow();
      setState((s) => ({ ...s, input: "", parts: [], isSubmitting: false, error: null }));
      return;
    }

    setState((s) => ({ ...s, isSubmitting: true, parts: [], error: null }));

    try {
      const streamParts: MessagePart[] = [];

      const response = await sendMessage(text, (event: StreamEvent) => {
        if (event.event !== "part.updated") return;
        const part = getPartFromEventData(event.data);
        if (!part) return;
        streamParts.push(part);
        setState((s) => ({ ...s, parts: [...streamParts] }));
      });

      setState((s) => ({
        ...s,
        input: "",
        isSubmitting: false,
        parts: response.reply_parts ?? [],
        error: null,
      }));
    } catch (err) {
      console.error("CommandBar submit error", err);
      setState((s) => ({
        ...s,
        isSubmitting: false,
        parts: [],
        error: normalizeErrorMessage(err),
      }));
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
