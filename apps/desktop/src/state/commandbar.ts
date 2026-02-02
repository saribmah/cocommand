import { useState, useCallback } from "react";
import { hideWindow, openSettingsWindow, type CoreResult } from "../lib/ipc";
import { useSessionStore } from "./session";
import type { MessagePart, StreamEvent, StreamPart } from "../types/session";

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
      const streamBlocks: StreamBlock[] = [];

      const sessionResult: CoreResult = {
        type: "preview",
        title: "Session",
        body: "",
      };
      setState((s) => ({
        ...s,
        results: [sessionResult],
      }));

      const response = await sendMessage(text, (event: StreamEvent) => {
        if (event.event === "part") {
          const part = (event.data as { part?: StreamPart })?.part;
          if (!part) return;
          applyStreamPart(part, streamBlocks);
          const body = buildStreamBody(streamBlocks);
          setState((s) => ({
            ...s,
            results: [
              {
                type: "preview",
                title: "Session",
                body,
              },
            ],
          }));
        }
      });

      const sessionBody = formatMessageParts(response.reply_parts);
      const finalResult: CoreResult | null = sessionBody
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
        results: finalResult ? [finalResult] : [],
        pendingConfirmation: null,
        followUpActive: false,
      }));
    } catch (err) {
      console.error("CommandBar submit error", err);
      const errorResult: CoreResult = {
        type: "error",
        title: "Error",
        body: normalizeErrorMessage(err),
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

function normalizeErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
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
          return `\n\n[ToolCall] ${part.tool_name} (running)`;
        case "tool-result":
          return `\n\n[ToolResult${part.is_error ? " Error" : ""}] ${
            part.tool_name
          } (${part.is_error ? "failed" : "ok"})`;
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

function formatSource(part: MessagePart & { type: "source" }): string {
  const bits = [];
  if (part.title) bits.push(part.title);
  if (part.url) bits.push(part.url);
  if (part.filename) bits.push(part.filename);
  return bits.join("\n");
}

type StreamBlockType =
  | "text"
  | "reasoning"
  | "tool"
  | "source"
  | "file";

interface StreamBlock {
  type: StreamBlockType;
  text: string;
}

function applyStreamPart(part: StreamPart, blocks: StreamBlock[]): void {
  switch (part.type) {
    case "text-delta": {
      const last = blocks[blocks.length - 1];
      if (!last || last.type !== "text") {
        blocks.push({ type: "text", text: part.text ?? "" });
      } else {
        last.text += part.text ?? "";
      }
      return;
    }
    case "reasoning-delta": {
      const last = blocks[blocks.length - 1];
      if (!last || last.type !== "reasoning") {
        blocks.push({ type: "reasoning", text: part.text ?? "" });
      } else {
        last.text += part.text ?? "";
      }
      return;
    }
    case "tool-call":
      blocks.push({
        type: "tool",
        text: `[ToolCall] ${part.toolName ?? part.tool_name ?? ""} (running)`,
      });
      return;
    case "tool-result":
      blocks.push({
        type: "tool",
        text: `[ToolResult] ${part.toolName ?? part.tool_name ?? ""} (ok)`,
      });
      return;
    case "tool-error":
      blocks.push({
        type: "tool",
        text: `[ToolResult Error] ${
          part.toolName ?? part.tool_name ?? ""
        } (failed)`,
      });
      return;
    case "source":
      blocks.push({
        type: "source",
        text: `[Source] ${part.sourceType ?? part.source_type ?? ""}\n${formatSourcePart(
          part
        )}`,
      });
      return;
    case "file":
      blocks.push({
        type: "file",
        text: `[File] ${part.mediaType ?? part.media_type ?? ""}${
          part.name ? ` (${part.name})` : ""
        }`,
      });
      return;
    default:
      return;
  }
}

function buildStreamBody(blocks: StreamBlock[]): string {
  return blocks
    .map((block) => {
      if (block.type === "reasoning") {
        return `\n\n[Reasoning]\n${block.text}`;
      }
      if (block.type === "text") {
        return block.text;
      }
      return `\n\n${block.text}`;
    })
    .join("");
}

function formatSourcePart(part: StreamPart): string {
  const bits: string[] = [];
  if (part.title) bits.push(String(part.title));
  if (part.url) bits.push(String(part.url));
  if (part.name) bits.push(String(part.name));
  return bits.join("\n");
}
