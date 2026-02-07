export interface SessionContext {
  workspace_id: string;
  session_id: string;
  started_at: number;
  ended_at: number | null;
}

export interface RecordMessageResponse {
  context: SessionContext;
  reply_parts: MessagePart[];
}

export interface StreamEvent<T = unknown> {
  event: string;
  data: T;
}

export interface MessagePartBase {
  id: string;
  sessionId: string;
  messageId: string;
}

export type MessagePart =
  | TextPart
  | ReasoningPart
  | ToolPart
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

export type ToolState = ToolStatePending | ToolStateRunning | ToolStateCompleted | ToolStateError;

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
