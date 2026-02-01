import { useState, useCallback } from "react";
import { hideWindow, openSettingsWindow, type CoreResult } from "../lib/ipc";
import { useSessionStore } from "./session";
import type { MessagePart } from "../types/session";

export interface CommandBarState {
  input: string;
  selectedIndex: number;
  isSubmitting: boolean;
  results: CoreResult[];
  pendingConfirmation: null;
  followUpActive: boolean;
}

export function useCommandBar() {
  const [state, setState] = useState<CommandBarState>({
    input: "",
    selectedIndex: -1,
    isSubmitting: false,
    results: [],
    pendingConfirmation: null,
    followUpActive: false,
  });

  const setInput = useCallback((value: string) => {
    setState((s) => ({ ...s, input: value }));
  }, []);

  const setResults = useCallback((results: CoreResult[]) => {
    setState((s) => ({ ...s, results }));
  }, []);

  const reset = useCallback(() => {
    setState({
      input: "",
      selectedIndex: -1,
      isSubmitting: false,
      results: [],
      pendingConfirmation: null,
      followUpActive: false,
    });
  }, []);

  const sendMessage = useSessionStore((store) => store.sendMessage);

  const submit = useCallback(async (override?: string) => {
    const text = (override ?? state.input).trim();
    if (!text) return;

    if (text === "/settings") {
      await openSettingsWindow();
      hideWindow();
      setState((s) => ({
        ...s,
        input: "",
        results: [],
        isSubmitting: false,
      }));
      return;
    }

    setState((s) => ({ ...s, isSubmitting: true }));

    try {
      const response = await sendMessage(text);
      const sessionBody = formatMessageParts(response.reply_parts);
      const sessionResult: CoreResult | null = sessionBody
        ? {
            type: "preview",
            title: "Session",
            body: sessionBody,
          }
        : null;
      setState((s) => ({
        ...s,
        input: "",
        isSubmitting: false,
        results: sessionResult ? [sessionResult] : [],
        pendingConfirmation: null,
        followUpActive: false,
      }));
    } catch (err) {
      const errorResult: CoreResult = {
        type: "error",
        title: "Error",
        body: String(err),
      };
      setState((s) => ({
        ...s,
        isSubmitting: false,
        results: [errorResult],
      }));
    }
  }, [state.input, sendMessage]);

  const dismissResult = useCallback((index: number) => {
    setState((s) => ({
      ...s,
      results: s.results.filter((_, i) => i !== index),
    }));
  }, []);

  const confirmPending = useCallback(async () => {}, []);
  const cancelPending = useCallback(async () => {}, []);

  const dismiss = useCallback(() => {
    if (state.pendingConfirmation) {
      cancelPending();
    } else if (state.results.length > 0) {
      setState((s) => ({ ...s, results: [] }));
    } else {
      hideWindow();
    }
  }, [state.pendingConfirmation, state.results.length, cancelPending]);

  return {
    ...state,
    setInput,
    setResults,
    submit,
    dismiss,
    dismissResult,
    confirmPending,
    cancelPending,
    reset,
  };
}

function formatMessageParts(parts: MessagePart[]): string {
  return parts
    .map((part) => {
      switch (part.type) {
        case "text":
          return part.text;
        case "reasoning":
          return `\n\n[Reasoning]\n${part.text}`;
        case "tool-call":
          return `\n\n[ToolCall] ${part.tool_name}\n${formatJson(part.input)}`;
        case "tool-result":
          return `\n\n[ToolResult${part.is_error ? " Error" : ""}] ${
            part.tool_name
          }\n${formatJson(part.output)}`;
        case "source":
          return `\n\n[Source] ${part.source_type}\n${formatSource(part)}`;
        case "file":
          return `\n\n[File] ${part.media_type}${part.name ? ` (${part.name})` : ""}`;
        default:
          return "";
      }
    })
    .join("");
}

function formatJson(value: unknown): string {
  try {
    return "```json\n" + JSON.stringify(value, null, 2) + "\n```";
  } catch {
    return String(value);
  }
}

function formatSource(part: MessagePart & { type: "source" }): string {
  const bits = [];
  if (part.title) bits.push(part.title);
  if (part.url) bits.push(part.url);
  if (part.filename) bits.push(part.filename);
  return bits.join("\n");
}
