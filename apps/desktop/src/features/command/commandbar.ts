import { useCallback, useState } from "react";
import { hideWindow, openSettingsWindow } from "../../lib/ipc";
import type { MessagePart, StreamEvent, StreamPart } from "../session/session.types";

interface PendingConfirmation {
  title: string;
  body: string;
  confirmation_id?: string;
}

export interface CommandBarState {
  input: string;
  selectedIndex: number;
  isSubmitting: boolean;
  parts: MessagePart[];
  pendingConfirmation: PendingConfirmation | null;
  followUpActive: boolean;
  error: string | null;
}

type SendMessageFn = (
  text: string,
  onEvent?: (event: StreamEvent) => void
) => Promise<{ reply_parts?: MessagePart[] }>;

export function useCommandBar(sendMessage: SendMessageFn) {
  const [state, setState] = useState<CommandBarState>({
    input: "",
    selectedIndex: -1,
    isSubmitting: false,
    parts: [],
    pendingConfirmation: null,
    followUpActive: false,
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
      selectedIndex: -1,
      isSubmitting: false,
      parts: [],
      pendingConfirmation: null,
      followUpActive: false,
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
        if (event.event === "part") {
          const part = (event.data as { part?: StreamPart })?.part;
          if (!part) return;
          applyStreamPart(part, streamParts);
          setState((s) => ({ ...s, parts: [...streamParts] }));
        }
      });

      setState((s) => ({
        ...s,
        input: "",
        isSubmitting: false,
        parts: response.reply_parts ?? [],
        pendingConfirmation: null,
        followUpActive: false,
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

  const confirmPending = useCallback(async () => {}, []);
  const cancelPending = useCallback(async () => {}, []);

  const dismiss = useCallback(() => {
    if (state.pendingConfirmation) {
      cancelPending();
    } else if (state.parts.length > 0 || state.error) {
      setState((s) => ({ ...s, parts: [], error: null }));
    } else {
      hideWindow();
    }
  }, [state.pendingConfirmation, state.parts.length, state.error, cancelPending]);

  return {
    ...state,
    setInput,
    setError,
    submit,
    dismiss,
    confirmPending,
    cancelPending,
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

function applyStreamPart(part: StreamPart, parts: MessagePart[]): void {
  switch (part.type) {
    case "text-delta": {
      const last = parts[parts.length - 1];
      if (!last || last.type !== "text") {
        parts.push({ type: "text", text: part.text ?? "" });
      } else {
        last.text += part.text ?? "";
      }
      return;
    }
    case "reasoning-delta": {
      const last = parts[parts.length - 1];
      if (!last || last.type !== "reasoning") {
        parts.push({ type: "reasoning", text: part.text ?? "" });
      } else {
        last.text += part.text ?? "";
      }
      return;
    }
    case "tool-call": {
      const toolName = part.toolName ?? part.tool_name ?? "tool";
      const callId = part.toolCallId ?? part.tool_call_id ?? `tool_${parts.length}`;
      parts.push({
        type: "tool-call",
        call_id: callId,
        tool_name: toolName,
        input: part.input ?? {},
      });
      return;
    }
    case "tool-result": {
      const toolName = part.toolName ?? part.tool_name ?? "tool";
      const callId = part.toolCallId ?? part.tool_call_id ?? `tool_${parts.length}`;
      parts.push({
        type: "tool-result",
        call_id: callId,
        tool_name: toolName,
        output: part.output ?? {},
        is_error: false,
      });
      return;
    }
    case "tool-error": {
      const toolName = part.toolName ?? part.tool_name ?? "tool";
      const callId = part.toolCallId ?? part.tool_call_id ?? `tool_${parts.length}`;
      parts.push({
        type: "tool-result",
        call_id: callId,
        tool_name: toolName,
        output: part.error ?? part.output ?? {},
        is_error: true,
      });
      return;
    }
    case "source": {
      const id = part.title ?? part.url ?? `source_${parts.length}`;
      parts.push({
        type: "source",
        id,
        source_type: part.sourceType ?? part.source_type ?? "source",
        url: part.url ?? null,
        title: part.title ?? null,
        media_type: part.mediaType ?? part.media_type ?? null,
        filename: part.name ?? null,
      });
      return;
    }
    case "file": {
      parts.push({
        type: "file",
        base64: "",
        media_type: part.mediaType ?? part.media_type ?? "",
        name: part.name ?? null,
      });
      return;
    }
    default:
      return;
  }
}
