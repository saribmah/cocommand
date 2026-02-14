import type { SessionContext } from "../session/session.types";

export interface RecordMessageResponse {
  context: SessionContext;
  reply_parts: MessagePart[];
}

export interface MessagePartBase {
  id: string;
  sessionId: string;
  messageId: string;
}

export interface TextSource {
  value: string;
  start: number;
  end: number;
}

export interface TextPartInput {
  type: "text";
  text: string;
}

export interface ExtensionPartInput {
  type: "extension";
  extensionId: string;
  name: string;
  kind?: string | null;
  source?: TextSource | null;
}

export type MessagePartInput = TextPartInput | ExtensionPartInput;

export type MessagePart =
  | TextPart
  | ReasoningPart
  | ToolPart
  | ExtensionPart
  | SourcePart
  | FilePart;

export interface TextPart extends MessagePartBase {
  type: "text";
  text: string;
}

export interface ReasoningPart extends MessagePartBase {
  type: "reasoning";
  text: string;
}

export type ToolState =
  | ToolStatePending
  | ToolStateRunning
  | ToolStateCompleted
  | ToolStateError;

export interface ToolStatePending {
  status: "pending";
  input: Record<string, unknown>;
  raw: string;
}

export interface ToolStateRunning {
  status: "running";
  input: Record<string, unknown>;
  title?: string | null;
  metadata?: Record<string, unknown> | null;
  time: {
    start: number;
  };
}

export interface ToolStateCompleted {
  status: "completed";
  input: Record<string, unknown>;
  output: string;
  title: string;
  metadata: Record<string, unknown>;
  time: {
    start: number;
    end: number;
    compacted?: number | null;
  };
  attachments?: FilePart[] | null;
}

export interface ToolStateError {
  status: "error";
  input: Record<string, unknown>;
  error: string;
  metadata?: Record<string, unknown> | null;
  time: {
    start: number;
    end: number;
  };
}

export interface ToolPart extends MessagePartBase {
  type: "tool";
  callId: string;
  tool: string;
  state: ToolState;
  metadata?: Record<string, unknown> | null;
}

export interface ExtensionPart extends MessagePartBase {
  type: "extension";
  extensionId: string;
  name: string;
  kind?: string | null;
  source?: TextSource | null;
}

export interface SourcePart extends MessagePartBase {
  type: "source";
  sourceId?: string | null;
  sourceType: string;
  url?: string | null;
  title?: string | null;
  mediaType?: string | null;
  filename?: string | null;
}

export interface FilePart extends MessagePartBase {
  type: "file";
  base64: string;
  mediaType: string;
  name?: string | null;
}
